//! The library asks for the same cover several times: React's StrictMode in
//! dev, and in every build several views (grid, Discover, Big Picture…)
//! mounting the same game at once. Each request used to run the whole cover
//! cascade, spending the user's iiDB allowance and hitting every source twice.
//!
//! - `Coalescer`: a request for a key already in flight waits for that
//!   request's result instead of starting its own.
//! - `MissMemo`: a game no source had is not searched again for a while
//!   (a forced refresh or clearing the cover cache searches again).

use std::collections::HashMap;
use std::future::Future;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tokio::sync::oneshot;

pub struct Coalescer<T> {
    waiting: Mutex<HashMap<String, Vec<oneshot::Sender<T>>>>,
}

/// Removes the key when the leading request ends, even if it was cancelled:
/// its waiters then see a closed channel and do the work themselves.
struct LeaderGuard<'a, T> {
    waiting: &'a Mutex<HashMap<String, Vec<oneshot::Sender<T>>>>,
    key: String,
}

impl<T> Drop for LeaderGuard<'_, T> {
    fn drop(&mut self) {
        if let Ok(mut w) = self.waiting.lock() {
            w.remove(&self.key);
        }
    }
}

impl<T: Clone> Coalescer<T> {
    pub fn new() -> Self {
        Self { waiting: Mutex::new(HashMap::new()) }
    }

    /// Runs `work` for `key`, or waits for the identical request in flight.
    pub async fn run<F: Future<Output = T>>(&self, key: String, work: F) -> T {
        let waiter = {
            let mut w = self.waiting.lock().unwrap_or_else(|e| e.into_inner());
            match w.get_mut(&key) {
                Some(list) => {
                    let (tx, rx) = oneshot::channel();
                    list.push(tx);
                    Some(rx)
                }
                None => {
                    w.insert(key.clone(), Vec::new());
                    None
                }
            }
        };
        if let Some(rx) = waiter {
            return match rx.await {
                Ok(shared) => shared,
                Err(_) => work.await,
            };
        }
        let guard = LeaderGuard { waiting: &self.waiting, key };
        let out = work.await;
        let waiters = guard.waiting.lock().unwrap_or_else(|e| e.into_inner()).remove(&guard.key).unwrap_or_default();
        for tx in waiters {
            let _ = tx.send(out.clone());
        }
        out
    }

    #[cfg(test)]
    fn in_flight(&self) -> usize {
        self.waiting.lock().unwrap().len()
    }
}

pub struct MissMemo {
    ttl: Duration,
    misses: Mutex<HashMap<String, Instant>>,
}

impl MissMemo {
    pub fn new(ttl: Duration) -> Self {
        Self { ttl, misses: Mutex::new(HashMap::new()) }
    }

    pub fn record(&self, key: &str, now: Instant) {
        if let Ok(mut m) = self.misses.lock() {
            m.retain(|_, at| now.saturating_duration_since(*at) < self.ttl);
            m.insert(key.to_string(), now);
        }
    }

    pub fn clear(&self) {
        if let Ok(mut m) = self.misses.lock() {
            m.clear();
        }
    }

    pub fn recently_missed(&self, key: &str, now: Instant) -> bool {
        self.misses
            .lock()
            .map(|m| m.get(key).is_some_and(|at| now.saturating_duration_since(*at) < self.ttl))
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    #[tokio::test]
    async fn identical_requests_in_flight_share_one_run() {
        let c = Arc::new(Coalescer::<Result<String, String>>::new());
        let runs = Arc::new(AtomicUsize::new(0));
        let call = |key: &str| {
            let (c, runs, key) = (c.clone(), runs.clone(), key.to_string());
            async move {
                c.run(key.clone(), async move {
                    runs.fetch_add(1, Ordering::SeqCst);
                    tokio::time::sleep(Duration::from_millis(50)).await;
                    Ok(format!("cover of {key}"))
                })
                .await
            }
        };
        let (a, b, other) = tokio::join!(call("Celeste|Switch"), call("Celeste|Switch"), call("Zelda|Switch"));
        assert_eq!(a, Ok("cover of Celeste|Switch".into()));
        assert_eq!(b, a, "the second request gets the first one's result");
        assert_eq!(other, Ok("cover of Zelda|Switch".into()));
        assert_eq!(runs.load(Ordering::SeqCst), 2, "one run per distinct game");
        assert_eq!(c.in_flight(), 0);
        // Once finished, a new request runs again (it will hit the disk cache).
        call("Celeste|Switch").await.unwrap();
        assert_eq!(runs.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn a_cancelled_leader_does_not_strand_its_waiters() {
        let c = Arc::new(Coalescer::<u32>::new());
        let leader = {
            let c = c.clone();
            tokio::spawn(async move { c.run("k".into(), std::future::pending::<u32>()).await })
        };
        tokio::time::sleep(Duration::from_millis(20)).await;
        let waiter = {
            let c = c.clone();
            tokio::spawn(async move { c.run("k".into(), async { 7 }).await })
        };
        tokio::time::sleep(Duration::from_millis(20)).await;
        leader.abort();
        let got = tokio::time::timeout(Duration::from_secs(2), waiter).await.expect("waiter hung").unwrap();
        assert_eq!(got, 7, "the waiter did the work itself");
        assert_eq!(c.in_flight(), 0);
    }

    #[test]
    fn misses_are_remembered_for_a_while_only() {
        let memo = MissMemo::new(Duration::from_secs(600));
        let t0 = Instant::now();
        assert!(!memo.recently_missed("NSR ROM|Switch", t0));
        memo.record("NSR ROM|Switch", t0);
        assert!(memo.recently_missed("NSR ROM|Switch", t0 + Duration::from_secs(599)));
        assert!(!memo.recently_missed("Other|Switch", t0));
        assert!(!memo.recently_missed("NSR ROM|Switch", t0 + Duration::from_secs(600)));
        memo.record("NSR ROM|Switch", t0);
        memo.clear();
        assert!(!memo.recently_missed("NSR ROM|Switch", t0), "clearing the cover cache searches again");
    }
}
