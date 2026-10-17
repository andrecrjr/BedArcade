export type Direction = 'up' | 'down' | 'left' | 'right';
const SELECTOR =
  'a[href],button,input:not([type="hidden"]),select,textarea,summary,[role="button"],[role="link"],[role="checkbox"],[role="radio"],[tabindex]:not([tabindex="-1"])';

export function candidates(): HTMLElement[] {
  return [...document.querySelectorAll<HTMLElement>(SELECTOR)].filter((el) => {
    if (el.closest('[inert]') || el.matches(':disabled,[aria-disabled="true"]')) return false;
    const style = getComputedStyle(el);
    const rect = el.getBoundingClientRect();
    return (
      style.display !== 'none' &&
      style.visibility !== 'hidden' &&
      rect.width > 0 &&
      rect.height > 0 &&
      rect.bottom > 0 &&
      rect.right > 0 &&
      rect.top < innerHeight &&
      rect.left < innerWidth
    );
  });
}

function center(el: HTMLElement) {
  const r = el.getBoundingClientRect();
  return { x: r.left + r.width / 2, y: r.top + r.height / 2 };
}

export function nearest(current: HTMLElement | null, direction: Direction): HTMLElement | null {
  const keyboard = current?.closest<HTMLElement>('[data-gamepad-navigator="keyboard"]') ?? null;
  const all = candidates().filter((candidate) => !keyboard || keyboard.contains(candidate));
  if (!all.length) return keyboard ? current : null;
  if (!current || !all.includes(current)) return all[0] ?? current;
  const from = center(current);
  let best: HTMLElement | null = null;
  let bestScore = Infinity;
  for (const el of all) {
    if (el === current) continue;
    const to = center(el);
    const dx = to.x - from.x;
    const dy = to.y - from.y;
    const primary =
      direction === 'left' ? -dx : direction === 'right' ? dx : direction === 'up' ? -dy : dy;
    const cross = direction === 'left' || direction === 'right' ? Math.abs(dy) : Math.abs(dx);
    if (primary <= 4) continue;
    const score = primary + cross * 2.5;
    if (score < bestScore) {
      bestScore = score;
      best = el;
    }
  }
  return best ?? (keyboard ? current : null);
}
