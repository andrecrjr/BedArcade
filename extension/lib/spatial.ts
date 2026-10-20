export type Direction = 'up' | 'down' | 'left' | 'right';
const SELECTOR =
  'a[href],button,input:not([type="hidden"]),select,textarea,summary,[role="button"],[role="link"],[role="checkbox"],[role="radio"],[role="menuitem"],[role="menuitemcheckbox"],[role="menuitemradio"],[role="option"],[role="tab"],[role="switch"],[role="slider"],[role="spinbutton"],[role="textbox"],[role="searchbox"],[role="combobox"],[role="treeitem"],[role="gridcell"],[tabindex]:not([tabindex="-1"])';
const LAYER_SELECTOR = 'dialog[open],[role="dialog"],[role="alertdialog"],[popover]';

type CandidateRect = { el: HTMLElement; rect: DOMRect };

function visibleRect(el: HTMLElement): DOMRect | null {
  if (el.closest('[inert],[aria-hidden="true"]') || el.matches(':disabled,[aria-disabled="true"]'))
    return null;
  const style = getComputedStyle(el);
  if (style.display === 'none' || style.visibility !== 'visible' || style.opacity === '0')
    return null;
  const rect = el.getBoundingClientRect();
  return rect.width > 0 &&
    rect.height > 0 &&
    rect.bottom > 0 &&
    rect.right > 0 &&
    rect.top < innerHeight &&
    rect.left < innerWidth
    ? rect
    : null;
}

function visible(el: HTMLElement): boolean {
  return visibleRect(el) !== null;
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

function candidateRects(keyboard?: HTMLElement | null): CandidateRect[] {
  const layer = activeLayer();
  const keyboardContainer = document.querySelector<HTMLElement>(
    '[data-gamepad-desktop="keyboard"]',
  );
  const result: CandidateRect[] = [];
  for (const el of document.querySelectorAll<HTMLElement>(SELECTOR)) {
    if (
      (layer && !layer.contains(el) && !keyboardContainer?.contains(el)) ||
      (keyboard && !keyboard.contains(el))
    ) {
      continue;
    }
    const rect = visibleRect(el);
    if (rect) result.push({ el, rect });
  }
  return result;
}

export function candidates(): HTMLElement[] {
  return candidateRects().map(({ el }) => el);
}

export function nearest(current: HTMLElement | null, direction: Direction): HTMLElement | null {
  const keyboard = current?.closest<HTMLElement>('[data-gamepad-desktop="keyboard"]') ?? null;
  const all = candidateRects(keyboard);
  if (!all.length) return keyboard ? current : null;
  const currentRect = all.find((candidate) => candidate.el === current)?.rect;
  if (!currentRect) return all[0]?.el ?? current;
  const fromX = currentRect.left + currentRect.width / 2;
  const fromY = currentRect.top + currentRect.height / 2;
  let best: HTMLElement | null = null;
  let bestScore = Infinity;
  for (const { el, rect } of all) {
    if (el === current) continue;
    const dx = rect.left + rect.width / 2 - fromX;
    const dy = rect.top + rect.height / 2 - fromY;
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
