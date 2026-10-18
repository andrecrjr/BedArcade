import {
  applyTranslations,
  detectAppLocale,
  loadAppLocale,
  watchAppLocale,
  type AppLocale,
  type TranslationKey,
} from './i18n';
import { createGamepadPrompt, watchGamepadIconChanges } from './gamepad-icons';
import {
  DEFAULT_GAMEPAD_BINDINGS,
  GAMEPAD_BINDINGS_STORAGE_KEY,
  GAMEPAD_START_BUTTON,
  loadGamepadBindings,
  normalizeGamepadBindings,
  type GamepadBindings,
} from './gamepad-bindings';

type TextField = HTMLInputElement | HTMLTextAreaElement;

const keyboardSubmitTargets = new WeakMap<HTMLElement, () => TextField | null>();

const TEXT_INPUT_TYPES = new Set(['email', 'number', 'password', 'search', 'tel', 'text', 'url']);

function isTextField(target: EventTarget | null): target is TextField {
  return (
    target instanceof HTMLTextAreaElement ||
    (target instanceof HTMLInputElement && TEXT_INPUT_TYPES.has(target.type))
  );
}

function textRange(target: TextField): [number, number] {
  try {
    const start = target.selectionStart;
    const end = target.selectionEnd;
    if (start !== null && end !== null) return [start, end];
  } catch {
    // Input types without selection support append at the end.
  }
  return [target.value.length, target.value.length];
}

function replaceText(target: TextField, start: number, end: number, text: string) {
  try {
    target.setRangeText(text, start, end, 'end');
  } catch {
    target.value = `${target.value.slice(0, start)}${text}${target.value.slice(end)}`;
  }
  target.dispatchEvent(new Event('input', { bubbles: true, composed: true }));
}

export function pressEnter(target: HTMLElement) {
  if (!isTextField(target)) return false;
  const keydown = new KeyboardEvent('keydown', {
    key: 'Enter',
    code: 'Enter',
    bubbles: true,
    cancelable: true,
  });
  if (target.dispatchEvent(keydown)) {
    if (target instanceof HTMLTextAreaElement) {
      const [start, end] = textRange(target);
      replaceText(target, start, end, '\n');
    } else target.form?.requestSubmit();
  }
  target.dispatchEvent(new KeyboardEvent('keyup', { key: 'Enter', code: 'Enter', bubbles: true }));
  return true;
}

export function submitForm(target: HTMLElement | null) {
  const keyboard = document.querySelector<HTMLElement>('[data-gamepad-navigator="keyboard"]');
  const keyboardTarget =
    keyboard && keyboard.style.display !== 'none' ? keyboardSubmitTargets.get(keyboard)?.() : null;
  const submitTarget = keyboardTarget ?? target;
  if (!submitTarget) return false;

  const form =
    submitTarget instanceof HTMLFormElement
      ? submitTarget
      : (submitTarget.closest('form') ??
        (submitTarget instanceof HTMLInputElement ||
        submitTarget instanceof HTMLButtonElement ||
        submitTarget instanceof HTMLSelectElement ||
        submitTarget instanceof HTMLTextAreaElement
          ? submitTarget.form
          : null));
  if (!form) return false;

  const isSubmitter =
    (submitTarget instanceof HTMLButtonElement && submitTarget.type === 'submit') ||
    (submitTarget instanceof HTMLInputElement &&
      (submitTarget.type === 'submit' || submitTarget.type === 'image'));
  if (isSubmitter && submitTarget.form === form && !submitTarget.disabled)
    form.requestSubmit(submitTarget);
  else form.requestSubmit();
  return true;
}

export function hideFloatingKeyboard() {
  const keyboard = document.querySelector<HTMLElement>('[data-gamepad-navigator="keyboard"]');
  if (!keyboard || keyboard.style.display === 'none') return false;
  const hideKey = keyboard.querySelector<HTMLButtonElement>('[data-key-action="hide"]');
  if (!hideKey) return false;
  hideKey.click();
  return true;
}

function createKey(
  label: string,
  key?: string,
  action?: string,
  translationKey?: TranslationKey,
  ariaTranslationKey?: TranslationKey,
) {
  const button = document.createElement('button');
  button.type = 'button';
  button.textContent = label;
  button.setAttribute('aria-label', label === '⌫' ? 'Backspace' : label === '⇧' ? 'Shift' : label);
  if (translationKey) button.dataset.i18n = translationKey;
  const labelKey = ariaTranslationKey ?? translationKey;
  if (labelKey) button.dataset.i18nAriaLabel = labelKey;
  if (key !== undefined) button.dataset.key = key;
  if (action) button.dataset.keyAction = action;
  button.addEventListener('mousedown', (event) => event.preventDefault());
  Object.assign(button.style, {
    display: 'grid',
    minWidth: '0',
    height: '34px',
    flex: '1',
    placeItems: 'center',
    border: '1px solid rgba(235,235,235,.24)',
    borderRadius: '8px',
    padding: '0 6px',
    background: 'rgba(230,230,230,.1)',
    color: '#e5edf5',
    font: '13px system-ui,sans-serif',
    cursor: 'pointer',
    pointerEvents: 'auto',
  });
  return button;
}

export function attachFloatingKeyboard({
  root = document.documentElement,
  urlShortcuts = false,
}: { root?: HTMLElement; urlShortcuts?: boolean } = {}) {
  let typingTarget: TextField | null = null;
  let hiddenTarget: TextField | null = null;
  let shiftEnabled = false;
  let positionFrame = 0;
  const keyboard = document.createElement('div');
  keyboard.dataset.gamepadNavigator = 'keyboard';
  keyboard.setAttribute('role', 'group');
  keyboard.setAttribute('aria-label', 'Floating keyboard');
  keyboard.dataset.i18nAriaLabel = 'keyboardGroup';
  Object.assign(keyboard.style, {
    position: 'fixed',
    left: '12px',
    top: '12px',
    zIndex: '2147483646',
    display: 'none',
    width: 'min(680px, calc(100vw - 24px))',
    maxHeight: 'min(55vh, 420px)',
    overflowY: 'auto',
    transform: 'none',
    boxSizing: 'border-box',
    border: '1px solid rgba(235,235,235,.3)',
    borderRadius: '18px',
    padding: '12px',
    background: 'linear-gradient(145deg, rgba(48,49,51,.9), rgba(29,30,32,.92))',
    backdropFilter: 'blur(18px) saturate(110%)',
    color: '#f5f8fc',
    boxShadow: '0 18px 48px rgba(0,0,0,.48), inset 0 1px 0 rgba(255,255,255,.12)',
    font: '14px system-ui,sans-serif',
    pointerEvents: 'none',
  });

  const header = document.createElement('div');
  Object.assign(header.style, {
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
    marginBottom: '8px',
    fontSize: '12px',
    fontWeight: '600',
  });
  const title = document.createElement('span');
  title.textContent = 'Keyboard';
  title.dataset.i18n = 'keyboardTitle';
  const hideKey = createKey('Hide', undefined, 'hide', 'keyboardHide');
  Object.assign(hideKey.style, { flex: '0 0 auto', height: '28px', fontSize: '11px' });
  header.append(title, hideKey);
  keyboard.append(header);

  const controllerHint = document.createElement('div');
  Object.assign(controllerHint.style, {
    display: 'flex',
    flexWrap: 'wrap',
    alignItems: 'center',
    gap: '8px 14px',
    marginBottom: '8px',
  });
  let gamepadBindings: GamepadBindings = DEFAULT_GAMEPAD_BINDINGS;
  let appLocale: AppLocale = detectAppLocale();
  let localeChanged = false;
  function renderControllerHint() {
    const hints = [
      { button: gamepadBindings.activate, labelKey: 'keyboardType' },
      { button: GAMEPAD_START_BUTTON, labelKey: 'hintSubmit' },
      { button: gamepadBindings.back, labelKey: 'keyboardHide' },
    ] satisfies { button: number; labelKey: TranslationKey }[];
    controllerHint.replaceChildren(
      ...hints.map(({ button, labelKey }) => {
        const hint = document.createElement('span');
        Object.assign(hint.style, { display: 'inline-flex', alignItems: 'center', gap: '4px' });
        const label = document.createElement('span');
        label.dataset.i18n = labelKey;
        Object.assign(label.style, { color: '#c4d1e4', font: '11px system-ui,sans-serif' });
        hint.append(createGamepadPrompt([button], false, appLocale), label);
        return hint;
      }),
    );
    applyTranslations(controllerHint, appLocale);
  }
  renderControllerHint();
  const stopGamepadIconWatch = watchGamepadIconChanges(renderControllerHint);
  keyboard.append(controllerHint);

  const focusStyle = document.createElement('style');
  focusStyle.textContent =
    '[data-gamepad-navigator="keyboard"] button:focus { outline: 3px solid #63e6be !important; outline-offset: 2px; background: rgba(99,230,190,.2) !important; box-shadow: 0 0 0 3px rgba(0,0,0,.4); }';
  keyboard.append(focusStyle);

  function addRow(keys: string[], { shift = false, backspace = false } = {}) {
    const row = document.createElement('div');
    Object.assign(row.style, {
      display: 'flex',
      justifyContent: 'center',
      gap: '4px',
      marginTop: '4px',
    });
    if (shift) row.append(createKey('⇧', undefined, 'shift', undefined, 'keyboardShift'));
    for (const key of keys) row.append(createKey(key, key));
    if (backspace)
      row.append(createKey('⌫', undefined, 'backspace', undefined, 'keyboardBackspace'));
    keyboard.append(row);
  }

  addRow('1234567890'.split(''));
  addRow('qwertyuiop'.split(''));
  addRow('asdfghjkl'.split(''));
  addRow('zxcvbnm'.split(''), { shift: true, backspace: true });
  addRow(['-', '_', '.', ',', '/', '?', '!', '@', '#', ':']);
  const utilityRow = document.createElement('div');
  Object.assign(utilityRow.style, { display: 'flex', gap: '4px', marginTop: '4px' });
  utilityRow.append(createKey('Clear', undefined, 'clear', 'keyboardClear'));
  const spaceKey = createKey('Space', ' ', undefined, 'keyboardSpace');
  Object.assign(spaceKey.style, { flex: '3' });
  utilityRow.append(spaceKey, createKey('Enter', undefined, 'enter', 'keyboardEnter'));
  keyboard.append(utilityRow);

  const urlShortcutRow = urlShortcuts ? document.createElement('div') : null;
  if (urlShortcutRow) {
    urlShortcutRow.dataset.urlShortcuts = '';
    urlShortcutRow.setAttribute('role', 'group');
    urlShortcutRow.setAttribute('aria-label', 'URL shortcuts');
    urlShortcutRow.dataset.i18nAriaLabel = 'urlShortcuts';
    Object.assign(urlShortcutRow.style, {
      display: 'none',
      flexWrap: 'wrap',
      gap: '4px',
      marginTop: '4px',
    });
    for (const shortcut of ['https://', 'www.', '.com', '.org', '.net']) {
      const key = createKey(shortcut, shortcut);
      Object.assign(key.style, { flex: '1 1 auto', minWidth: '68px' });
      urlShortcutRow.append(key);
    }
    keyboard.append(urlShortcutRow);
  }

  function resetShift() {
    shiftEnabled = false;
    keyboard.querySelector('[data-key-action="shift"]')?.setAttribute('aria-pressed', 'false');
    keyboard.querySelectorAll<HTMLButtonElement>('[data-key]').forEach((button) => {
      if (/^[a-z]$/i.test(button.dataset.key ?? ''))
        button.textContent = button.dataset.key!.toLowerCase();
    });
  }

  function positionNearTarget() {
    if (!typingTarget) return;

    const field = typingTarget.getBoundingClientRect();
    const panel = keyboard.getBoundingClientRect();
    const gap = 12;
    const edge = 12;
    const maxLeft = Math.max(edge, innerWidth - panel.width - edge);
    const left = Math.min(maxLeft, Math.max(edge, field.left + field.width / 2 - panel.width / 2));
    const maxTop = Math.max(edge, innerHeight - panel.height - edge);
    const below = field.bottom + gap;
    const preferredTop =
      below + panel.height <= innerHeight - edge ? below : field.top - panel.height - gap;
    const top = Math.min(maxTop, Math.max(edge, preferredTop));

    keyboard.style.left = `${left}px`;
    keyboard.style.top = `${top}px`;
  }

  function schedulePosition() {
    if (!typingTarget || keyboard.style.display === 'none') return;
    cancelAnimationFrame(positionFrame);
    positionFrame = requestAnimationFrame(() => {
      positionFrame = 0;
      positionNearTarget();
    });
  }

  function dismiss() {
    cancelAnimationFrame(positionFrame);
    positionFrame = 0;
    typingTarget = null;
    resetShift();
    if (urlShortcutRow) urlShortcutRow.style.display = 'none';
    keyboard.style.display = 'none';
  }

  function show(target: TextField) {
    hiddenTarget = null;
    typingTarget = target;
    if (urlShortcutRow) {
      urlShortcutRow.style.display =
        target instanceof HTMLInputElement && target.inputMode === 'url' ? 'flex' : 'none';
    }
    keyboard.style.display = 'block';
    schedulePosition();
  }

  keyboardSubmitTargets.set(keyboard, () => typingTarget);

  function onFocusIn(event: FocusEvent) {
    if (isTextField(event.target)) {
      if (event.target !== hiddenTarget) show(event.target);
    } else if (!(event.target instanceof Node) || !keyboard.contains(event.target)) {
      hiddenTarget = null;
      dismiss();
    }
  }

  function onFocusOut() {
    queueMicrotask(() => {
      const active = document.activeElement;
      if (!isTextField(active) && !(active instanceof Node && keyboard.contains(active))) dismiss();
    });
  }

  function onPointerDown(event: PointerEvent) {
    if (isTextField(event.target)) {
      if (event.target === hiddenTarget) show(event.target);
      return;
    }
    if (!(event.target instanceof Node) || !keyboard.contains(event.target)) {
      dismiss();
    }
  }

  function onClick(event: MouseEvent) {
    const button = event.target instanceof HTMLElement ? event.target.closest('button') : null;
    if (!button || !typingTarget) return;
    const action = button.dataset.keyAction;

    if (action === 'hide') {
      const target = typingTarget;
      hiddenTarget = target;
      dismiss();
      if (document.activeElement instanceof HTMLElement) document.activeElement.blur();
      target?.focus({ preventScroll: true });
    } else if (action === 'shift') {
      shiftEnabled = !shiftEnabled;
      button.setAttribute('aria-pressed', String(shiftEnabled));
      keyboard.querySelectorAll<HTMLButtonElement>('[data-key]').forEach((keyButton) => {
        const key = keyButton.dataset.key ?? '';
        if (/^[a-z]$/i.test(key)) keyButton.textContent = shiftEnabled ? key.toUpperCase() : key;
      });
    } else if (action === 'backspace') {
      const [start, end] = textRange(typingTarget);
      replaceText(typingTarget, start === end && start > 0 ? start - 1 : start, end, '');
    } else if (action === 'clear') {
      replaceText(typingTarget, 0, typingTarget.value.length, '');
    } else if (action === 'enter') {
      pressEnter(typingTarget);
    } else if (button.dataset.key !== undefined) {
      const key = button.dataset.key;
      const text = shiftEnabled && /^[a-z]$/i.test(key) ? key.toUpperCase() : key;
      const [start, end] = textRange(typingTarget);
      replaceText(typingTarget, start, end, text);
      if (shiftEnabled) resetShift();
    }
  }

  root.append(keyboard);
  let bindingsChanged = false;
  const onBindingsChanged = (
    changes: { [key: string]: chrome.storage.StorageChange },
    areaName: string,
  ) => {
    const change = changes[GAMEPAD_BINDINGS_STORAGE_KEY];
    if (areaName !== 'local' || !change) return;
    bindingsChanged = true;
    gamepadBindings = normalizeGamepadBindings(change.newValue);
    renderControllerHint();
  };
  try {
    chrome.storage.onChanged.addListener(onBindingsChanged);
  } catch {}
  void loadGamepadBindings()
    .then((bindings) => {
      if (bindingsChanged) return;
      gamepadBindings = bindings;
      renderControllerHint();
    })
    .catch(() => {});
  const stopLocaleWatch = watchAppLocale((locale) => {
    localeChanged = true;
    appLocale = locale;
    applyTranslations(keyboard, locale);
  });
  void loadAppLocale().then((locale) => {
    if (!localeChanged) {
      appLocale = locale;
      applyTranslations(keyboard, locale);
    }
  });
  document.addEventListener('focusin', onFocusIn);
  document.addEventListener('focusout', onFocusOut);
  document.addEventListener('pointerdown', onPointerDown, true);
  window.addEventListener('resize', schedulePosition);
  window.addEventListener('scroll', schedulePosition, true);
  keyboard.addEventListener('click', onClick);

  return () => {
    cancelAnimationFrame(positionFrame);
    stopGamepadIconWatch();
    document.removeEventListener('focusin', onFocusIn);
    document.removeEventListener('focusout', onFocusOut);
    document.removeEventListener('pointerdown', onPointerDown, true);
    window.removeEventListener('resize', schedulePosition);
    window.removeEventListener('scroll', schedulePosition, true);
    keyboard.removeEventListener('click', onClick);
    stopLocaleWatch();
    try {
      chrome.storage.onChanged.removeListener(onBindingsChanged);
    } catch {}
    keyboard.remove();
  };
}
