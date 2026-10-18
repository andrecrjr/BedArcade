export type Direction = 'up' | 'down' | 'left' | 'right';
const SELECTOR =
  'a[href],button,input:not([type="hidden"]),select,textarea,summary,[role="button"],[role="link"],[role="checkbox"],[role="radio"],[role="menuitem"],[role="menuitemcheckbox"],[role="menuitemradio"],[role="option"],[role="tab"],[role="switch"],[role="slider"],[role="spinbutton"],[role="textbox"],[role="searchbox"],[role="combobox"],[role="treeitem"],[role="gridcell"],[tabindex]:not([tabindex="-1"])';
const LAYER_SELECTOR = 'dialog[open],[role="dialog"],[role="alertdialog"],[popover]';

function visible(el: HTMLElement): boolean {
  if (el.closest('[inert],[aria-hidden="true"]') || el.matches(':disabled,[aria-disabled="true"]'))
    return false;
  const style = getComputedStyle(el);
  const rect = el.getBoundingClientRect();
  return (
    style.display !== 'none' &&
    style.visibility === 'visible' &&
    style.opacity !== '0' &&
    rect.width > 0 &&
    rect.height > 0 &&
    rect.bottom > 0 &&
    rect.right > 0 &&
    rect.top < innerHeight &&
    rect.left < innerWidth
  );
}

function openLayer(el: HTMLElement): boolean {
  if (el instanceof HTMLDialogElement) return el.open;
  if (el.hasAttribute('popover')) {
    try {
      return el.matches(':popover-open');
    } catch {
      return false;
    }
  }
  return el.getAttribute('aria-modal') !== 'false';
}

function activeLayer(): HTMLElement | null {
  const layers = [...document.querySelectorAll<HTMLElement>(LAYER_SELECTOR)].filter(
    (el) => openLayer(el) && visible(el),
  );
  const focusedLayer =
    document.activeElement instanceof HTMLElement
      ? document.activeElement.closest<HTMLElement>(LAYER_SELECTOR)
      : null;
  return (
    (focusedLayer && layers.includes(focusedLayer) ? focusedLayer : null) ?? layers.at(-1) ?? null
  );
}

export function candidates(): HTMLElement[] {
  const layer = activeLayer();
  const keyboard = document.querySelector<HTMLElement>('[data-gamepad-navigator="keyboard"]');
  return [...document.querySelectorAll<HTMLElement>(SELECTOR)].filter(
    (el) => (!layer || layer.contains(el) || keyboard?.contains(el)) && visible(el),
  );
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
