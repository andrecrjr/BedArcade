import { candidates, nearest, type Direction } from '../lib/spatial';
import { startGamepadController } from '../lib/gamepad';
import { attachFloatingKeyboard, hideFloatingKeyboard } from '../lib/floating-keyboard';
import { showSelectPicker } from '../lib/select-picker';
import { sendTabAction } from '../lib/tab-actions';
import {
  detectAppLocale,
  loadAppLocale,
  translate,
  watchAppLocale,
  type AppLocale,
} from '../lib/i18n';

export default defineContentScript({
  matches: ['http://*/*', 'https://*/*'],
  runAt: 'document_idle',
  main() {
    // Evita executar em iframes e duplicar entradas do mesmo gamepad.
    if (window !== window.top) return;
    let enabled = true;
    let connected = false;
    let appLocale: AppLocale = detectAppLocale();
    let languageChanged = false;
    let selected: HTMLElement | null = null;
    let activeSelectPicker: HTMLDialogElement | null = null;
    let frame = 0;

    const outline = document.createElement('div');
    outline.dataset.gamepadNavigator = 'outline';
    Object.assign(outline.style, {
      position: 'fixed',
      pointerEvents: 'none',
      zIndex: '2147483647',
      border: '3px solid #63e6be',
      borderRadius: '7px',
      boxShadow: '0 0 0 3px rgba(0,0,0,.35)',
      transition: 'left 80ms,top 80ms,width 80ms,height 80ms',
      display: 'none',
      boxSizing: 'border-box',
    });
    document.documentElement.appendChild(outline);

    const status = document.createElement('div');
    status.dataset.gamepadNavigator = 'status';
    Object.assign(status.style, {
      position: 'fixed',
      right: '12px',
      bottom: '12px',
      zIndex: '2147483647',
      background: '#17212b',
      color: 'white',
      padding: '7px 10px',
      borderRadius: '7px',
      font: '12px system-ui',
      pointerEvents: 'none',
      display: 'none',
    });
    function renderStatus() {
      status.textContent = translate(
        appLocale,
        enabled ? 'controllerActive' : 'controllerStatePaused',
      );
    }

    renderStatus();
    const stopLocaleWatch = watchAppLocale((locale) => {
      languageChanged = true;
      appLocale = locale;
      renderStatus();
    });
    void loadAppLocale().then((locale) => {
      if (languageChanged) return;
      appLocale = locale;
      renderStatus();
    });
    document.documentElement.appendChild(status);

    const stopFloatingKeyboard = attachFloatingKeyboard();

    if (import.meta.env.MODE === 'e2e') {
      window.addEventListener('message', (event: MessageEvent<unknown>) => {
        const message = event.data;
        if (
          event.source === window &&
          typeof message === 'object' &&
          message !== null &&
          'type' in message &&
          message.type === 'gamepad-navigator:e2e-tab-action' &&
          'action' in message &&
          (message.action === 'prev-tab' || message.action === 'next-tab')
        ) {
          sendTabAction(message.action);
        }
      });
    }

    function draw() {
      if (!connected || !enabled || !selected || !selected.isConnected) {
        outline.style.display = 'none';
        return;
      }
      const r = selected.getBoundingClientRect();
      if (r.width <= 0 || r.height <= 0 || r.bottom < 0 || r.top > innerHeight) {
        outline.style.display = 'none';
        return;
      }
      Object.assign(outline.style, {
        display: 'block',
        left: `${r.left - 4}px`,
        top: `${r.top - 4}px`,
        width: `${r.width + 8}px`,
        height: `${r.height + 8}px`,
      });
    }

    document.addEventListener('focusin', (event) => {
      const target = event.target;
      if (target instanceof HTMLElement) {
        selected = target;
        draw();
      }
    });

    function move(direction: Direction) {
      if (document.activeElement instanceof HTMLElement && document.activeElement !== document.body)
        selected = document.activeElement;
      if (activeSelectPicker?.open) {
        const target = nearest(selected, direction);
        if (target) {
          selected = target;
          target.focus({ preventScroll: true });
          draw();
        }
        return;
      }
      const keyboard = document.querySelector<HTMLElement>('[data-gamepad-navigator="keyboard"]');
      if (
        direction === 'down' &&
        keyboard &&
        keyboard.style.display !== 'none' &&
        !keyboard.contains(selected)
      ) {
        const firstKey = keyboard.querySelector<HTMLButtonElement>('[data-key="q"]');
        if (firstKey) {
          selected = firstKey;
          firstKey.focus({ preventScroll: true });
          draw();
          return;
        }
      }

      const target = nearest(selected, direction);
      if (target) {
        selected = target;
        target.focus({ preventScroll: true });
        draw();
      } else {
        window.scrollBy({
          top: direction === 'down' ? 250 : direction === 'up' ? -250 : 0,
          left: direction === 'right' ? 250 : direction === 'left' ? -250 : 0,
          behavior: 'smooth',
        });
      }
    }

    function openSelectPicker(select: HTMLSelectElement) {
      const picker = showSelectPicker(select, appLocale);
      if (!picker) return;
      activeSelectPicker = picker.dialog;
      picker.dialog.addEventListener(
        'close',
        () => {
          if (activeSelectPicker === picker.dialog) activeSelectPicker = null;
          if (select.isConnected) {
            selected = select;
            select.focus({ preventScroll: true });
          } else selected = candidates()[0] ?? null;
          draw();
        },
        { once: true },
      );
      selected = picker.initialOption;
    }

    function activate() {
      if (!selected || !selected.isConnected) return;
      const target = selected;
      target.focus({ preventScroll: true });
      target.click();
      if (target instanceof HTMLSelectElement && target.isConnected) openSelectPicker(target);
    }

    selected = candidates()[0] ?? null;
    window.addEventListener('scroll', draw, { passive: true });
    window.addEventListener('resize', draw);
    const stopGamepad = startGamepadController({
      onAction(action) {
        if (action.type === 'move') move(action.direction);
        else if (action.type === 'scroll') window.scrollBy(0, action.amount);
        else if (action.type === 'activate') activate();
        else if (action.type === 'back') {
          if (activeSelectPicker?.open) activeSelectPicker.close('cancel');
          else if (hideFloatingKeyboard()) return;
          else sendTabAction('back');
        } else if (action.type === 'refresh') window.location.reload();
        else if (action.type === 'previous-tab') sendTabAction('prev-tab');
        else if (action.type === 'next-tab') sendTabAction('next-tab');
        else if (action.type === 'new-tab') sendTabAction('new-tab');
        if (!selected || !selected.isConnected) selected = candidates()[0] ?? null;
        draw();
      },
      onStateChange(state) {
        connected = state.connected;
        enabled = state.enabled;
        if (!state.connected) {
          status.style.display = 'none';
          draw();
          return;
        }
        renderStatus();
        status.style.display = 'block';
        draw();
      },
    });
    frame = requestAnimationFrame(draw);
    window.addEventListener(
      'pagehide',
      () => {
        stopGamepad();
        if (activeSelectPicker?.open) activeSelectPicker.close('cancel');
        cancelAnimationFrame(frame);
        stopLocaleWatch();
        outline.remove();
        status.remove();
        stopFloatingKeyboard();
      },
      { once: true },
    );
  },
});
