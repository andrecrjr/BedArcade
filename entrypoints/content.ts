import { candidates, nearest, type Direction } from '../lib/spatial';
import { startGamepadController } from '../lib/gamepad';
import { sendTabAction } from '../lib/tab-actions';

export default defineContentScript({
  matches: ['http://*/*', 'https://*/*'],
  runAt: 'document_idle',
  main() {
    // Evita executar em iframes e duplicar entradas do mesmo gamepad.
    if (window !== window.top) return;
    let enabled = true;
    let connected = false;
    let selected: HTMLElement | null = null;
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
    status.textContent = 'Gamepad ativo · Start: pausar';
    document.documentElement.appendChild(status);

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

    function move(direction: Direction) {
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

    function activate() {
      if (!selected || !selected.isConnected) return;
      selected.focus({ preventScroll: true });
      if (selected instanceof HTMLInputElement && ['checkbox', 'radio'].includes(selected.type)) {
        selected.click();
      } else if (selected instanceof HTMLSelectElement) {
        selected.focus();
      } else {
        selected.click();
      }
    }

    selected = candidates()[0] ?? null;
    window.addEventListener('scroll', draw, { passive: true });
    window.addEventListener('resize', draw);
    const stopGamepad = startGamepadController({
      onAction(action) {
        if (action.type === 'move') move(action.direction);
        else if (action.type === 'scroll') window.scrollBy(0, action.amount);
        else if (action.type === 'activate') activate();
        else if (action.type === 'back') sendTabAction('back');
        else if (action.type === 'previous-tab') sendTabAction('prev-tab');
        else if (action.type === 'next-tab') sendTabAction('next-tab');
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
        status.textContent = state.enabled
          ? 'Gamepad ativo · Start: pausar'
          : 'Gamepad pausado · Start: retomar';
        status.style.display = 'block';
        draw();
      },
    });
    frame = requestAnimationFrame(draw);
    window.addEventListener(
      'pagehide',
      () => {
        stopGamepad();
        cancelAnimationFrame(frame);
        outline.remove();
        status.remove();
      },
      { once: true },
    );
  },
});
