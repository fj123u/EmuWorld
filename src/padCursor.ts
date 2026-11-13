// The mouse pointer has no business on screen while the app is driven with a
// controller (couch, Big Picture): it is hidden at the first pad input and
// comes back as soon as the mouse really moves or clicks.
export const PAD_INPUT_CLASS = "pad-input";

/** How far (px) the mouse must travel to count as "the user took the mouse". */
const MOUSE_WAKE_DISTANCE = 6;
/** A stick resting off-centre must not count as input. */
const STICK_THRESHOLD = 0.5;

/** Whether a "gamepad-state" snapshot shows the pad being used. */
export function padIsUsed(buttons: boolean[] | undefined, axes: number[] | undefined): boolean {
  return (buttons ?? []).some(Boolean) || (axes ?? []).some((a) => Math.abs(a) > STICK_THRESHOLD);
}

export function hideCursorForPad() {
  document.documentElement.classList.add(PAD_INPUT_CLASS);
}

export function showCursor() {
  document.documentElement.classList.remove(PAD_INPUT_CLASS);
}

/**
 * Shows the pointer again when the mouse is used. Returns the cleanup.
 * A mousemove alone is not enough: the browser sends some when the page
 * scrolls under a still pointer, which pad navigation does all the time.
 */
export function showCursorOnMouseUse(): () => void {
  let anchor: { x: number; y: number } | null = null;
  const onMove = (e: MouseEvent) => {
    if (!document.documentElement.classList.contains(PAD_INPUT_CLASS)) {
      anchor = null;
      return;
    }
    if (!anchor) {
      anchor = { x: e.screenX, y: e.screenY };
      return;
    }
    if (Math.hypot(e.screenX - anchor.x, e.screenY - anchor.y) >= MOUSE_WAKE_DISTANCE) {
      anchor = null;
      showCursor();
    }
  };
  const onUse = () => {
    anchor = null;
    showCursor();
  };
  window.addEventListener("mousemove", onMove);
  window.addEventListener("mousedown", onUse);
  window.addEventListener("wheel", onUse, { passive: true });
  return () => {
    window.removeEventListener("mousemove", onMove);
    window.removeEventListener("mousedown", onUse);
    window.removeEventListener("wheel", onUse);
    showCursor();
  };
}
