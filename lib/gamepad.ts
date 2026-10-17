import type { Direction } from './spatial';

export type GamepadFrame = {
  id: string;
  connected: boolean;
  axes: readonly number[];
  buttons: readonly { pressed: boolean }[];
};

export const E2E_GAMEPAD_MESSAGE = 'gamepad-navigator:e2e-gamepad';

export type GamepadAction =
  | { type: 'move'; direction: Direction }
  | { type: 'activate' }
  | { type: 'back' }
  | { type: 'previous-tab' }
  | { type: 'next-tab' }
  | { type: 'scroll'; amount: number };

export interface GamepadState {
  connected: boolean;
  enabled: boolean;
}

export interface GamepadControllerOptions {
  onAction: (action: GamepadAction) => void;
  onStateChange: (state: GamepadState) => void;
}

type E2EMessage = { type: typeof E2E_GAMEPAD_MESSAGE; frame: GamepadFrame | null };

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null;
}

function isGamepadFrame(value: unknown): value is GamepadFrame {
  if (!isRecord(value) || !Array.isArray(value.axes) || !Array.isArray(value.buttons)) return false;
  return (
    typeof value.id === 'string' &&
    typeof value.connected === 'boolean' &&
    value.axes.every((axis) => typeof axis === 'number') &&
    value.buttons.every((button) => isRecord(button) && typeof button.pressed === 'boolean')
  );
}

function isE2EMessage(value: unknown): value is E2EMessage {
  return (
    isRecord(value) &&
    value.type === E2E_GAMEPAD_MESSAGE &&
    (value.frame === null || isGamepadFrame(value.frame))
  );
}

function createE2EReader(): () => GamepadFrame | null {
  let frame: GamepadFrame | null = null;
  window.addEventListener('message', (event: MessageEvent<unknown>) => {
    if (event.source === window && isE2EMessage(event.data)) frame = event.data.frame;
  });
  return () => (frame?.connected ? frame : null);
}

export function createGamepadReader(): () => GamepadFrame | null {
  if (import.meta.env.MODE === 'e2e') return createE2EReader();
  return () =>
    navigator.getGamepads().find((pad): pad is Gamepad => pad !== null && pad.connected) ?? null;
}

export function startGamepadController({ onAction, onStateChange }: GamepadControllerOptions) {
  const readGamepad = createGamepadReader();
  let frame = 0;
  let enabled = true;
  let connected = false;
  let previousButtons: boolean[] = [];
  let previousGamepadId = '';
  let lastMove = -Infinity;

  function tick(now: number) {
    const pad = readGamepad();

    if (!pad) {
      if (connected) onStateChange({ connected: false, enabled });
      connected = false;
      previousButtons = [];
      previousGamepadId = '';
      lastMove = -Infinity;
    } else {
      if (!connected) onStateChange({ connected: true, enabled });
      connected = true;

      if (pad.id !== previousGamepadId) {
        previousGamepadId = pad.id;
        previousButtons = pad.buttons.map((button) => button.pressed);
        lastMove = -Infinity;
      }

      const pressed = (index: number) => Boolean(pad.buttons[index]?.pressed);
      const edge = (index: number) => pressed(index) && !previousButtons[index];

      if (edge(9)) {
        enabled = !enabled;
        onStateChange({ connected: true, enabled });
      }

      if (enabled) {
        const x = pad.axes[0] ?? 0;
        const y = pad.axes[1] ?? 0;
        let direction: Direction | null = null;

        if (pressed(12)) direction = 'up';
        else if (pressed(13)) direction = 'down';
        else if (pressed(14)) direction = 'left';
        else if (pressed(15)) direction = 'right';
        else if (Math.abs(x) > 0.55 || Math.abs(y) > 0.55) {
          direction =
            Math.abs(x) > Math.abs(y) ? (x > 0 ? 'right' : 'left') : y > 0 ? 'down' : 'up';
        }

        if (direction && now - lastMove > 190) {
          onAction({ type: 'move', direction });
          lastMove = now;
        }

        const scrollY = pad.axes[3] ?? 0;
        if (Math.abs(scrollY) > 0.25) onAction({ type: 'scroll', amount: scrollY * 15 });
        if (edge(0)) onAction({ type: 'activate' });
        if (edge(1)) onAction({ type: 'back' });
        if (edge(4)) onAction({ type: 'previous-tab' });
        if (edge(5)) onAction({ type: 'next-tab' });
      }

      previousButtons = pad.buttons.map((button) => button.pressed);
    }

    frame = requestAnimationFrame(tick);
  }

  frame = requestAnimationFrame(tick);
  return () => cancelAnimationFrame(frame);
}
