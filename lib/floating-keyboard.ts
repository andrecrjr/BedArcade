type TextField = HTMLInputElement | HTMLTextAreaElement;

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

function createKey(label: string, key?: string, action?: string) {
  const button = document.createElement('button');
  button.type = 'button';
  button.textContent = label;
  button.setAttribute('aria-label', label === '⌫' ? 'Backspace' : label === '⇧' ? 'Shift' : label);
  if (key !== undefined) button.dataset.key = key;
  if (action) button.dataset.keyAction = action;
  button.addEventListener('mousedown', (event) => event.preventDefault());
  Object.assign(button.style, {
    display: 'grid',
    minWidth: '0',
    height: '34px',
    flex: '1',
    placeItems: 'center',
    border: '1px solid rgba(160,183,210,.2)',
    borderRadius: '6px',
    padding: '0 6px',
    background: 'rgba(137,158,184,.12)',
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
  Object.assign(keyboard.style, {
    position: 'fixed',
    left: '12px',
    top: '12px',
    zIndex: '2147483647',
    display: 'none',
    width: 'min(680px, calc(100vw - 24px))',
    maxHeight: 'min(55vh, 420px)',
    overflowY: 'auto',
    transform: 'none',
    boxSizing: 'border-box',
    border: '1px solid rgba(164,184,210,.28)',
    borderRadius: '14px',
    padding: '12px',
    background: 'rgba(15,25,38,.97)',
    color: '#f5f8fc',
    boxShadow: '0 12px 36px rgba(0,0,0,.45)',
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
  const hideKey = createKey('Hide', undefined, 'hide');
  Object.assign(hideKey.style, { flex: '0 0 auto', height: '28px', fontSize: '11px' });
  header.append(title, hideKey);
  keyboard.append(header);

  function addRow(keys: string[], { shift = false, backspace = false } = {}) {
    const row = document.createElement('div');
    Object.assign(row.style, {
      display: 'flex',
      justifyContent: 'center',
      gap: '4px',
      marginTop: '4px',
    });
    if (shift) row.append(createKey('⇧', undefined, 'shift'));
    for (const key of keys) row.append(createKey(key, key));
    if (backspace) row.append(createKey('⌫', undefined, 'backspace'));
    keyboard.append(row);
  }

  addRow('1234567890'.split(''));
  addRow('qwertyuiop'.split(''));
  addRow('asdfghjkl'.split(''));
  addRow('zxcvbnm'.split(''), { shift: true, backspace: true });
  addRow(['-', '_', '.', ',', '/', '?', '!', '@', '#', ':']);
  const utilityRow = document.createElement('div');
  Object.assign(utilityRow.style, { display: 'flex', gap: '4px', marginTop: '4px' });
  utilityRow.append(createKey('Clear', undefined, 'clear'));
  const spaceKey = createKey('Space', ' ');
  Object.assign(spaceKey.style, { flex: '3' });
  utilityRow.append(spaceKey, createKey('Enter', undefined, 'enter'));
  keyboard.append(utilityRow);

  const urlShortcutRow = urlShortcuts ? document.createElement('div') : null;
  if (urlShortcutRow) {
    urlShortcutRow.dataset.urlShortcuts = '';
    urlShortcutRow.setAttribute('role', 'group');
    urlShortcutRow.setAttribute('aria-label', 'URL shortcuts');
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
      if (typingTarget instanceof HTMLTextAreaElement) {
        const [start, end] = textRange(typingTarget);
        replaceText(typingTarget, start, end, '\n');
      } else {
        typingTarget.form?.requestSubmit();
      }
    } else if (button.dataset.key !== undefined) {
      const key = button.dataset.key;
      const text = shiftEnabled && /^[a-z]$/i.test(key) ? key.toUpperCase() : key;
      const [start, end] = textRange(typingTarget);
      replaceText(typingTarget, start, end, text);
      if (shiftEnabled) resetShift();
    }
  }

  root.append(keyboard);
  document.addEventListener('focusin', onFocusIn);
  document.addEventListener('focusout', onFocusOut);
  document.addEventListener('pointerdown', onPointerDown, true);
  window.addEventListener('resize', schedulePosition);
  window.addEventListener('scroll', schedulePosition, true);
  keyboard.addEventListener('click', onClick);

  return () => {
    cancelAnimationFrame(positionFrame);
    document.removeEventListener('focusin', onFocusIn);
    document.removeEventListener('focusout', onFocusOut);
    document.removeEventListener('pointerdown', onPointerDown, true);
    window.removeEventListener('resize', schedulePosition);
    window.removeEventListener('scroll', schedulePosition, true);
    keyboard.removeEventListener('click', onClick);
    keyboard.remove();
  };
}
