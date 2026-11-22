// Social → Challenges (v2): the challenges of the day, the week and the month,
// computed from the session journal (src/challenges.ts), with points, the
// weekly streak, badges and the challenges done. Nothing to join: playing
// moves them forward.
import { Award, Calendar, Flame, Trophy } from "lucide-react";
import { describe, progressText, type Challenge, type Period, type Rewards, type Rhythm } from "./challenges";
import { formatPlaytime } from "./home";
import type { RankEntry } from "./challengeSync";

type T = (key: string, params?: Record<string, string | number>) => string;

export interface ChallengeItem {
  challenge: Challenge;
  period: Period;
  value: number;
}

const RHYTHMS: Rhythm[] = ["day", "week", "month"];
const TIER_ICON = { bronze: "🥉", silver: "🥈", gold: "🥇" } as const;

/** "encore 3 j" / "encore 5 h" until the end of the period. */
export function timeLeft(period: Period, now: Date, t: T): string {
  const ms = Math.max(0, period.end.getTime() - now.getTime());
  const hours = Math.ceil(ms / 3_600_000);
  return hours > 24 ? t("challenges2.leftDays", { n: Math.ceil(hours / 24) }) : t("challenges2.leftHours", { n: hours });
}

/** "on": ranked with friends; "signedOut"; "unavailable": the server tables are not there yet. */
export type RankingState = "on" | "signedOut" | "unavailable";

interface Props {
  items: ChallengeItem[];
  rewards: Rewards;
  now: Date;
  t: T;
  /** Friends ranking of each common challenge, by challenge key. */
  ranks?: Record<string, RankEntry[]>;
  rankingState?: RankingState;
}

export function ChallengesPage({ items, rewards, now, t, ranks = {}, rankingState = "signedOut" }: Props) {
  const titleOf = (c: Challenge) => describe(c, t, formatPlaytime);
  return (
    <div className="challenges-v2">
      <div className="challenges-v2__summary">
        <span className="v3-pill"><Trophy size={14} /> {t("challenges2.totalPoints", { n: rewards.points })}</span>
        <span className="v3-pill"><Flame size={14} /> {rewards.streak > 0 ? t("challenges2.streak", { n: rewards.streak }) : t("challenges2.noStreak")}</span>
      </div>

      {RHYTHMS.map((rhythm) => {
        const list = items.filter((i) => i.challenge.rhythm === rhythm);
        if (list.length === 0) return null;
        return (
          <section key={rhythm} className="challenges-v2__section">
            <h2 className="challenges-v2__heading">
              {t(`challenges2.rhythm.${rhythm}`)}
              <span className="challenges-v2__left"><Calendar size={13} /> {timeLeft(list[0].period, now, t)}</span>
            </h2>
            <div className="challenges-v2__grid">
              {list.map(({ challenge: c, value }) => {
                const { title, description } = titleOf(c);
                const done = value >= c.goal;
                return (
                  <article key={c.key} className={`v3-card challenge-v2 gamepad-nav-item${done ? " challenge-v2--done" : ""}`} tabIndex={0}>
                    <div className="challenge-v2__head">
                      <h3 className="challenge-v2__title">{title}</h3>
                      <span className="v3-pill challenge-v2__points">{t("challenges2.points", { n: c.points })}</span>
                    </div>
                    <p className="challenge-v2__desc">{description}</p>
                    <div className="v3-progress" role="progressbar" aria-label={title} aria-valuemin={0} aria-valuemax={c.goal} aria-valuenow={value}>
                      <span style={{ width: `${Math.min(100, (value / Math.max(1, c.goal)) * 100)}%` }} />
                    </div>
                    <div className="challenge-v2__foot">
                      <span>{done ? t("challenges2.done") : progressText(c, value, formatPlaytime)}</span>
                      <span className="challenge-v2__family">{t(c.family === "common" ? "challenges2.common" : "challenges2.personal")}</span>
                    </div>
                    {rankingState === "on" && c.family === "common" && c.rhythm !== "day" && (
                      <div className="challenge-v2__ranking">
                        <span className="challenge-v2__ranking-title"><Trophy size={12} /> {t("challenges2.rankingTitle")}</span>
                        {(ranks[c.key] ?? []).length <= 1 ? (
                          <span className="home-card__muted">{t("challenges2.rankingEmpty")}</span>
                        ) : (
                          <ol>
                            {(ranks[c.key] ?? []).slice(0, 5).map((r, i) => (
                              <li key={r.userId} className={r.me ? "challenge-v2__rank--me" : undefined}>
                                <span>{i === 0 ? "🥇" : i === 1 ? "🥈" : i === 2 ? "🥉" : `#${i + 1}`}</span>
                                <span className="challenge-v2__rank-name">{r.name}</span>
                                <span>{r.done ? "✓" : progressText(c, r.progress, formatPlaytime)}</span>
                              </li>
                            ))}
                          </ol>
                        )}
                      </div>
                    )}
                  </article>
                );
              })}
            </div>
          </section>
        );
      })}

      <section className="challenges-v2__section">
        <h2 className="challenges-v2__heading"><Award size={16} /> {t("challenges2.badges")}</h2>
        <div className="challenges-v2__badges">
          {rewards.badges.map((b) => (
            <div key={b.kind} className={`v3-card challenges-v2__badge${b.tier ? "" : " challenges-v2__badge--locked"}`}>
              <span className="challenges-v2__badge-icon" aria-hidden="true">{b.tier ? TIER_ICON[b.tier] : "🔒"}</span>
              <span className="challenges-v2__badge-name">{t(`challenges2.kind.${b.kind}`)}</span>
              <span className="home-card__muted">{b.tier ? `${t(`challenges2.tier.${b.tier}`)} · ` : ""}{t("challenges2.badgeCount", { n: b.count })}</span>
            </div>
          ))}
        </div>
      </section>

      <section className="challenges-v2__section">
        <h2 className="challenges-v2__heading">{t("challenges2.history")}</h2>
        {rewards.recent.length === 0 ? (
          <p className="home-card__muted">{t("challenges2.noHistory")}</p>
        ) : (
          <ul className="challenges-v2__history">
            {rewards.recent.map((c) => (
              <li key={c.key}>
                <span>{t(`challenges2.kind.${c.kind}`)}</span>
                <span className="home-card__muted">{t(`challenges2.rhythm.${c.rhythm}`)} · {new Date(c.at).toLocaleDateString()}</span>
                <span className="v3-pill">+{c.points}</span>
              </li>
            ))}
          </ul>
        )}
        {rankingState !== "on" && (
          <p className="home-card__muted">{t(rankingState === "signedOut" ? "challenges2.rankingSignedOut" : "challenges2.rankingUnavailable")}</p>
        )}
      </section>
    </div>
  );
}
