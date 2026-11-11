// Social → "Activité récente": the friends' events (up to 50 from Supabase),
// shown a page at a time so the list never runs to hundreds of lines.
import { useState } from "react";

type T = (key: string, params?: Record<string, string | number>) => string;

export interface ActivityItem {
  id: string;
  event_type: string;
  game_name: string | null;
  console: string | null;
  details: string | null;
  created_at: string;
  username?: string;
  avatar_url?: string | null;
}

/** Events shown at first, and added by each "Voir plus". */
export const ACTIVITY_PAGE = 8;

function timeAgo(t: T, iso: string): string {
  const diff = Date.now() - new Date(iso).getTime();
  if (diff < 60000) return t("friendsUi.justNow");
  if (diff < 3600000) return t("friendsUi.minutesAgo", { n: Math.floor(diff / 60000) });
  if (diff < 86400000) return t("friendsUi.hoursAgo", { n: Math.floor(diff / 3600000) });
  return t("friendsUi.daysAgo", { n: Math.floor(diff / 86400000) });
}

export function ActivityFeed({ items, t }: { items: ActivityItem[]; t: T }) {
  const [shown, setShown] = useState(ACTIVITY_PAGE);
  const visible = items.slice(0, shown);
  return (
    <>
      <div className="activity-feed">
        {visible.map((a) => (
          <div key={a.id} className="activity-feed__item">
            <div className="activity-feed__avatar">
              {a.avatar_url ? (
                <img src={a.avatar_url} alt="" />
              ) : (
                <div className="friend-card__avatar-placeholder" style={{ width: 28, height: 28, fontSize: 10 }}>
                  {(a.username || "?").slice(0, 2).toUpperCase()}
                </div>
              )}
            </div>
            <div className="activity-feed__content">
              <span className="activity-feed__text">
                <strong>{a.username}</strong>{" "}
                {a.event_type === "game_started" && <>{t("friendsUi.activityStarted")} <em>{a.game_name}</em></>}
                {a.event_type === "achievement_unlocked" && <>{t("friendsUi.activityUnlocked")} {a.details}</>}
                {a.event_type === "game_added" && <>{t("friendsUi.activityAdded")} <em>{a.game_name}</em> {t("friendsUi.activityAddedEnd")}</>}
                {a.event_type === "game_completed" && <>{t("friendsUi.activityCompleted")} <em>{a.game_name}</em></>}
              </span>
              <span className="activity-feed__time">
                {timeAgo(t, a.created_at)}
                {a.console && ` · ${a.console}`}
              </span>
            </div>
          </div>
        ))}
      </div>
      {(items.length > shown || shown > ACTIVITY_PAGE) && (
        <div className="activity-feed__more">
          {items.length > shown && (
            <button className="btn btn--ghost btn--sm gamepad-nav-item" onClick={() => setShown(shown + ACTIVITY_PAGE)}>
              {t("friendsUi.activityMore", { count: Math.min(ACTIVITY_PAGE, items.length - shown) })}
            </button>
          )}
          {shown > ACTIVITY_PAGE && (
            <button className="btn btn--ghost btn--sm gamepad-nav-item" onClick={() => setShown(ACTIVITY_PAGE)}>
              {t("friendsUi.activityLess")}
            </button>
          )}
        </div>
      )}
    </>
  );
}
