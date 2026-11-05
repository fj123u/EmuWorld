// Full-screen celebration shown when an in-app achievement unlocks: gold glow,
// the badge popping in, and a short burst of confetti. Several unlocks at once
// are shown one after the other.
import { useEffect, useMemo, useState } from "react";

export interface CelebratedAchievement {
  id: string;
  name: string;
  description: string;
  icon: string;
  hidden: boolean;
}

export const ACHIEVEMENT_ANIMATION_KEY = "emuworld_achievement_animation";
/** How long one celebration stays on screen once the window is visible. */
export const CELEBRATION_MS = 4_000;

export function achievementAnimationEnabled(): boolean {
  try {
    return localStorage.getItem(ACHIEVEMENT_ANIMATION_KEY) !== "off";
  } catch {
    return true;
  }
}

const CONFETTI_COLORS = ["#f5c542", "#ffd86b", "#fff3c4", "#f0932b", "#ffffff"];
const CONFETTI_COUNT = 48;

interface Props {
  queue: CelebratedAchievement[];
  /** Removes the achievement currently shown (the head of the queue). */
  onDone: () => void;
  t: (key: string) => string;
}

export function AchievementCelebration({ queue, onDone, t }: Props) {
  const current = queue[0];
  // The timer only runs while the window is visible: an unlock that happens
  // while the emulator is in front waits for the player to come back.
  const [visible, setVisible] = useState(() => typeof document === "undefined" || !document.hidden);

  useEffect(() => {
    const update = () => setVisible(!document.hidden);
    document.addEventListener("visibilitychange", update);
    return () => document.removeEventListener("visibilitychange", update);
  }, []);

  useEffect(() => {
    if (!current || !visible) return;
    const timer = setTimeout(onDone, CELEBRATION_MS);
    return () => clearTimeout(timer);
  }, [current, visible, onDone]);

  useEffect(() => {
    if (!current) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape" || e.key === "Enter") onDone();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [current, onDone]);

  // Random but stable for one achievement, so a re-render doesn't reshuffle.
  const confetti = useMemo(
    () =>
      Array.from({ length: CONFETTI_COUNT }, (_, i) => ({
        left: Math.random() * 100,
        delay: Math.random() * 0.6,
        duration: 2.2 + Math.random() * 1.6,
        drift: (Math.random() - 0.5) * 240,
        spin: (Math.random() - 0.5) * 1440,
        color: CONFETTI_COLORS[i % CONFETTI_COLORS.length],
        wide: i % 3 === 0,
      })),
    [current?.id],
  );

  if (!current) return null;

  return (
    <div className="achievement-celebration" role="alertdialog" aria-labelledby="achievement-celebration-name" onClick={onDone}>
      <div className="achievement-celebration__confetti" aria-hidden="true">
        {confetti.map((c, i) => (
          <span
            key={i}
            className={`achievement-celebration__piece ${c.wide ? "achievement-celebration__piece--wide" : ""}`}
            style={{
              left: `${c.left}%`,
              background: c.color,
              animationDelay: `${c.delay}s`,
              animationDuration: `${c.duration}s`,
              ["--drift" as string]: `${c.drift}px`,
              ["--spin" as string]: `${c.spin}deg`,
            }}
          />
        ))}
      </div>
      <div className="achievement-celebration__card" key={current.id}>
        <div className="achievement-celebration__kicker">
          {t(current.hidden ? "celebration.secret" : "celebration.title")}
        </div>
        <div className="achievement-celebration__badge" aria-hidden="true">{current.icon}</div>
        <div className="achievement-celebration__name" id="achievement-celebration-name">{current.name}</div>
        <div className="achievement-celebration__desc">{current.description}</div>
        <button className="btn btn--primary gamepad-nav-item achievement-celebration__close" onClick={(e) => { e.stopPropagation(); onDone(); }}>
          {queue.length > 1 ? t("celebration.next") : t("celebration.continue")}
        </button>
      </div>
    </div>
  );
}
