// A versus between two friends (Social → Amis), scored by the challenges
// engine (challengeSync.ts): both scores, the time left, the result once
// closed, and a rematch.
import { RotateCcw, Swords } from "lucide-react";
import { versusRules, type Versus } from "./challengeSync";
import { formatPlaytime } from "./home";

type T = (key: string, params?: Record<string, string | number>) => string;

export interface VersusView extends Versus {
  goal_description: string;
  challenger_profile?: { username: string; avatar_url: string };
  opponent_profile?: { username: string; avatar_url: string };
}

interface Props {
  v: VersusView;
  me: string | undefined;
  t: T;
  now: Date;
  onAccept: (id: string) => void;
  onDecline: (id: string) => void;
  onRematch: (v: VersusView) => void;
}

export function VersusCard({ v, me, t, now, onAccept, onDecline, onRematch }: Props) {
  const rules = versusRules(v);
  const isChallenger = v.challenger_id === me;
  const mine = isChallenger ? v.challenger_progress : v.opponent_progress;
  const theirs = isChallenger ? v.opponent_progress : v.challenger_progress;
  const opponent = (isChallenger ? v.opponent_profile : v.challenger_profile)?.username || t("versusUi.opponent");
  const score = (n: number) => (rules.kind === "time" ? formatPlaytime(n) : String(n));
  const finished = v.status === "completed";
  const result = !finished ? null : v.winner_id === null ? t("versusV2.draw") : v.winner_id === me ? t("versusV2.youWon") : t("versusV2.theyWon", { name: opponent });
  const msLeft = v.ends_at ? Date.parse(v.ends_at) - now.getTime() : 0;
  const hoursLeft = Math.ceil(msLeft / 3_600_000);

  return (
    <div className={`versus-card gamepad-nav-item${finished ? " versus-card--done" : ""}`} tabIndex={0}>
      <div className="versus-card__header">
        <Swords size={14} />
        <span className="versus-card__type">{t(`versusV2.kind.${rules.kind}`)}</span>
        {(rules.game || rules.console) && <span className="versus-card__game">{rules.game ?? rules.console}</span>}
      </div>
      <p className="versus-card__desc">{v.goal_description}</p>
      <div className="versus-card__scores">
        <div className={`versus-card__player${mine > theirs ? " versus-card__player--ahead" : ""}`}>
          <span>{t("friendsUi.you")}</span>
          <strong>{score(mine)}</strong>
        </div>
        <span className="versus-card__vs">{t("friendsUi.vs")}</span>
        <div className={`versus-card__player${theirs > mine ? " versus-card__player--ahead" : ""}`}>
          <span>{opponent}</span>
          <strong>{score(theirs)}</strong>
        </div>
      </div>
      {v.status === "pending" && v.opponent_id === me && (
        <div className="versus-card__actions">
          <button className="btn btn--success btn--sm gamepad-nav-item" onClick={() => onAccept(v.id)}>{t("versus.accept")}</button>
          <button className="btn btn--ghost btn--sm gamepad-nav-item" onClick={() => onDecline(v.id)}>{t("versus.decline")}</button>
        </div>
      )}
      {v.status === "pending" && v.challenger_id === me && <div className="versus-card__timer">{t("versusV2.waiting", { name: opponent })}</div>}
      {v.status === "active" && v.ends_at && (
        <div className="versus-card__timer">
          {msLeft > 0
            ? (hoursLeft > 24 ? t("challenges2.leftDays", { n: Math.ceil(hoursLeft / 24) }) : t("challenges2.leftHours", { n: hoursLeft }))
            : t("versusV2.closing")}
        </div>
      )}
      {finished && (
        <div className="versus-card__actions">
          <strong className="versus-card__result">{result}</strong>
          <button className="btn btn--ghost btn--sm gamepad-nav-item" onClick={() => onRematch(v)}><RotateCcw size={12} /> {t("versusV2.rematch")}</button>
        </div>
      )}
    </div>
  );
}
