//! What goes into the daily log files (`%LOCALAPPDATA%/EmuWorld/logs/`):
//!
//! - every line is cleaned of secrets (API keys, tokens, e-mail addresses)
//!   before it reaches the file, whatever the caller wrote;
//! - a line repeated back to back is written once, then "répété N fois";
//! - each start writes a header: version, system, folders, what is set up;
//! - a Rust panic is written with where it happened;
//! - `diagnostic_report` gathers the header and the end of the log, with the
//!   user's folder name hidden, for the user to send when asking for help.

use regex::Regex;
use std::sync::OnceLock;

fn re(cell: &'static OnceLock<Regex>, pattern: &str) -> &'static Regex {
    cell.get_or_init(|| Regex::new(pattern).expect("valid log regex"))
}

/// Hides secrets in a log line: e-mail addresses, bearer tokens, JWTs,
/// `key=` / `token=` style values and API key headers.
pub fn redact(line: &str) -> String {
    static EMAIL: OnceLock<Regex> = OnceLock::new();
    static BEARER: OnceLock<Regex> = OnceLock::new();
    static JWT: OnceLock<Regex> = OnceLock::new();
    static PARAM: OnceLock<Regex> = OnceLock::new();
    static HEADER: OnceLock<Regex> = OnceLock::new();
    let s = re(&EMAIL, r"[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}").replace_all(line, "<e-mail>");
    // Headers first: "Authorization: Bearer x" is one secret, not two.
    let s = re(&HEADER, r"(?i)\b(x-api-key|authorization|apikey)\s*:\s*(?:bearer\s+)?\S+").replace_all(&s, "$1: <masqué>");
    let s = re(&BEARER, r"(?i)\bbearer\s+[A-Za-z0-9._~+/=-]{8,}").replace_all(&s, "Bearer <masqué>");
    let s = re(&JWT, r"\beyJ[A-Za-z0-9_-]{8,}\.[A-Za-z0-9_-]{8,}\.[A-Za-z0-9_-]{8,}").replace_all(&s, "<jeton>");
    let s = re(&PARAM, r"(?i)\b(api[_-]?key|access_token|refresh_token|token|key|password|secret|code)=([^&\s]+)").replace_all(&s, "$1=<masqué>");
    s.into_owned()
}

/// Back-to-back repeats of one line are written once, then counted.
#[derive(Default)]
pub struct Deduper {
    last: Option<(String, String)>,
    repeats: u32,
}

impl Deduper {
    /// The lines to write for this message (level, text): none while it
    /// repeats the previous one; before a new one, how often the last repeated.
    pub fn feed(&mut self, level: &str, msg: &str) -> Vec<(String, String)> {
        if let Some((l, m)) = &self.last {
            if l == level && m == msg {
                self.repeats += 1;
                return vec![];
            }
        }
        let mut out = vec![];
        if self.repeats > 0 {
            if let Some((l, _)) = &self.last {
                out.push((l.clone(), format!("↑ ligne précédente répétée {} fois de plus", self.repeats)));
            }
        }
        self.repeats = 0;
        self.last = Some((level.to_string(), msg.to_string()));
        out.push((level.to_string(), msg.to_string()));
        out
    }
}

/// What the start-up header says. Built in lib.rs from the real config.
pub struct StartupInfo {
    pub version: String,
    pub system: String,
    pub portable: bool,
    pub data_dir: String,
    pub roms_dir: String,
    pub emulators_dir: String,
    pub covers_dir: String,
    pub config_found: bool,
    pub installed_emulators: Vec<String>,
    pub iidb_key: bool,
    pub sgdb_key: bool,
    pub retroachievements: bool,
    pub bandwidth_limit_kbps: u64,
}

pub fn startup_lines(i: &StartupInfo) -> Vec<String> {
    let yes = |b: bool| if b { "oui" } else { "non" };
    vec![
        format!("===== EmuWorld {} démarré =====", i.version),
        format!("Système: {}", i.system),
        format!("Mode portable: {} — données: {}", yes(i.portable), i.data_dir),
        format!("Dossiers: ROMs {} · émulateurs {} · covers {}", i.roms_dir, i.emulators_dir, i.covers_dir),
        format!("Config: {}", if i.config_found { "trouvée" } else { "absente (valeurs par défaut)" }),
        format!(
            "Émulateurs installés ({}): {}",
            i.installed_emulators.len(),
            if i.installed_emulators.is_empty() { "aucun".to_string() } else { i.installed_emulators.join(", ") }
        ),
        format!(
            "Clé iiDB: {} · clé SteamGridDB: {} · RetroAchievements: {} · limite de débit: {}",
            yes(i.iidb_key),
            yes(i.sgdb_key),
            yes(i.retroachievements),
            if i.bandwidth_limit_kbps == 0 { "aucune".to_string() } else { format!("{} ko/s", i.bandwidth_limit_kbps) }
        ),
    ]
}

/// A Rust panic, as one log line: what and where.
pub fn panic_line(message: &str, location: Option<(&str, u32)>) -> String {
    match location {
        Some((file, line)) => format!("Panique Rust: {} ({}:{})", message, file, line),
        None => format!("Panique Rust: {}", message),
    }
}

/// The report for "Copier le rapport de diagnostic": the header lines, then
/// the end of the log, with the user's home folder shown as %USERPROFILE%
/// (it holds their Windows user name) and secrets hidden again.
pub fn diagnostic_report(header: &[String], log_tail: &str, home: Option<&str>) -> String {
    let mut text = String::from("EmuWorld — rapport de diagnostic\n");
    for line in header {
        text.push_str(line);
        text.push('\n');
    }
    text.push_str("\n----- Fin du log -----\n");
    text.push_str(log_tail);
    if let Some(home) = home.filter(|h| h.len() > 3) {
        text = text.replace(home, "%USERPROFILE%");
        // Logs write paths with either separator.
        text = text.replace(&home.replace('\\', "/"), "%USERPROFILE%");
    }
    text.lines().map(redact).collect::<Vec<_>>().join("\n")
}

/// The last `max_lines` lines of `text`.
pub fn tail(text: &str, max_lines: usize) -> String {
    let lines: Vec<&str> = text.lines().collect();
    lines[lines.len().saturating_sub(max_lines)..].join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hides_secrets_in_any_log_line() {
        assert_eq!(redact("Connecté: jean.dupont@gmail.com"), "Connecté: <e-mail>");
        assert_eq!(redact("Authorization: Bearer abcdef123456789"), "Authorization: <masqué>");
        assert_eq!(redact("en-tête Bearer abcdef123456789xyz"), "en-tête Bearer <masqué>");
        assert_eq!(redact("GET https://x.io/a?key=SECRET123&page=2"), "GET https://x.io/a?key=<masqué>&page=2");
        assert_eq!(redact("retour emuworld://cb?code=abc123"), "retour emuworld://cb?code=<masqué>");
        assert_eq!(redact("X-API-Key: 0123456789abcdef"), "X-API-Key: <masqué>");
        assert_eq!(redact("jeton eyJhbGciOiJIUzI1NiJ9.eyJyb2xlIjoiYW5vbiJ9.cfZhqeB8IntRnefIS3Dir ok"), "jeton <jeton> ok");
        // Ordinary lines are left alone.
        let plain = "Fetch cover: 'Pokémon HeartGold' (Nintendo DS) [force=false] → https://dl3.vimm.net/";
        assert_eq!(redact(plain), plain);
    }

    #[test]
    fn writes_a_repeated_line_once_then_how_often() {
        let mut d = Deduper::default();
        assert_eq!(d.feed("INFO", "a").len(), 1);
        assert!(d.feed("INFO", "a").is_empty());
        assert!(d.feed("INFO", "a").is_empty());
        let next = d.feed("WARN", "b");
        assert_eq!(next, vec![
            ("INFO".to_string(), "↑ ligne précédente répétée 2 fois de plus".to_string()),
            ("WARN".to_string(), "b".to_string()),
        ]);
        // Same text at another level is another line.
        assert_eq!(d.feed("INFO", "b").len(), 1);
    }

    #[test]
    fn the_startup_header_says_what_is_set_up_without_secrets() {
        let lines = startup_lines(&StartupInfo {
            version: "3.1.1".into(),
            system: "Windows 10.0.26200 (x86_64)".into(),
            portable: false,
            data_dir: r"C:\Users\jujua\AppData\Local\EmuWorld".into(),
            roms_dir: r"D:\Roms".into(),
            emulators_dir: r"D:\Emus".into(),
            covers_dir: r"D:\Covers".into(),
            config_found: false,
            installed_emulators: vec!["mesen".into(), "rpcs3".into()],
            iidb_key: true,
            sgdb_key: false,
            retroachievements: false,
            bandwidth_limit_kbps: 0,
        });
        let text = lines.join("\n");
        assert!(lines[0].contains("EmuWorld 3.1.1 démarré"));
        assert!(text.contains("Config: absente (valeurs par défaut)"));
        assert!(text.contains("Émulateurs installés (2): mesen, rpcs3"));
        assert!(text.contains("Clé iiDB: oui · clé SteamGridDB: non"));
    }

    #[test]
    fn the_diagnostic_report_hides_the_user_name_and_secrets() {
        let header = vec!["===== EmuWorld 3.1.1 démarré =====".to_string()];
        let log = "[2026-11-23 16:49:06] [INFO] Scan ROMs démarré dans: C:\\Users\\jujua\\AppData\\Local\\EmuWorld\\ROMs\n\
                   [2026-11-23 16:49:07] [INFO] mail jujua@example.com";
        let report = diagnostic_report(&header, log, Some(r"C:\Users\jujua"));
        assert!(report.contains(r"%USERPROFILE%\AppData\Local\EmuWorld\ROMs"));
        assert!(!report.contains("jujua"));
        assert!(report.starts_with("EmuWorld — rapport de diagnostic"));
        assert_eq!(tail("1\n2\n3\n4", 2), "3\n4");
        assert_eq!(panic_line("boom", Some(("src/lib.rs", 12))), "Panique Rust: boom (src/lib.rs:12)");
    }
}
