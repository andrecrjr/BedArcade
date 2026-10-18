import type { Direction } from './spatial';
import {
  DEFAULT_GAMEPAD_BINDINGS,
  GAMEPAD_BINDINGS_STORAGE_KEY,
  GAMEPAD_SELECT_BUTTON,
  GAMEPAD_START_BUTTON,
  loadGamepadBindings,
  normalizeGamepadBindings,
  type GamepadBindings,
} from './gamepad-bindings';

const TAB_CHORD_WINDOW_MS = 180;

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
  | { type: 'submit' }
  | { type: 'back' }
  | { type: 'refresh' }
  | { type: 'new-tab' }
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
  let stopped = false;
  let enabled = true;
  let connected = false;
  let bindings: GamepadBindings = DEFAULT_GAMEPAD_BINDINGS;
  let bindingsReady = false;
  let settingsChanged = false;
  let ignoreButtonEdges = false;
  let pendingNextTabAt: number | null = null;
  let previousButtons: boolean[] = [];
  let previousGamepadId = '';
  let lastMove = -Infinity;
  const onStorageChanged = (
    changes: { [key: string]: chrome.storage.StorageChange },
    areaName: string,
  ) => {
    const change = changes[GAMEPAD_BINDINGS_STORAGE_KEY];
    if (areaName !== 'local' || !change) return;
    bindings = normalizeGamepadBindings(change.newValue);
    bindingsReady = true;
    settingsChanged = true;
    ignoreButtonEdges = true;
    pendingNextTabAt = null;
  };

  try {
    chrome.storage.onChanged.addListener(onStorageChanged);
  } catch {}
  void loadGamepadBindings()
    .then((savedBindings) => {
      if (settingsChanged) return;
      bindings = savedBindings;
      ignoreButtonEdges = true;
    })
    .catch(() => {})
    .finally(() => {
      bindingsReady = true;
      if (!stopped) frame = requestAnimationFrame(tick);
    });

  function tick(now: number) {
    const pad = readGamepad();

    if (!pad) {
      if (connected) onStateChange({ connected: false, enabled });
      connected = false;
      previousButtons = [];
      previousGamepadId = '';
      pendingNextTabAt = null;
      lastMove = -Infinity;
    } else {
      if (!connected) onStateChange({ connected: true, enabled });
      connected = true;

      if (pad.id !== previousGamepadId) {
        previousGamepadId = pad.id;
        previousButtons = pad.buttons.map((button) => button.pressed);
        pendingNextTabAt = null;
        lastMove = -Infinity;
      }

      const pressed = (index: number) => Boolean(pad.buttons[index]?.pressed);
      const edge = (index: number) =>
        !ignoreButtonEdges && pressed(index) && !previousButtons[index];

      if (bindingsReady && edge(GAMEPAD_SELECT_BUTTON)) {
        enabled = !enabled;
        if (!enabled) pendingNextTabAt = null;
        onStateChange({ connected: true, enabled });
      }

      if (enabled && bindingsReady) {
        const x = pad.axes[0] ?? 0;
        const y = pad.axes[1] ?? 0;
        let direction: Direction | null = null;

        if (pressed(bindings.moveUp)) direction = 'up';
        else if (pressed(bindings.moveDown)) direction = 'down';
        else if (pressed(bindings.moveLeft)) direction = 'left';
        else if (pressed(bindings.moveRight)) direction = 'right';
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
        if (edge(bindings.activate)) onAction({ type: 'activate' });
        if (edge(GAMEPAD_START_BUTTON)) onAction({ type: 'submit' });
        if (edge(bindings.back)) onAction({ type: 'back' });
        if (edge(bindings.previousTab)) onAction({ type: 'previous-tab' });

        const refreshChord =
          pressed(bindings.refreshModifier) &&
          pressed(bindings.refreshTrigger) &&
          (edge(bindings.refreshModifier) || edge(bindings.refreshTrigger));
        if (refreshChord) {
          pendingNextTabAt = null;
          onAction({ type: 'refresh' });
        } else if (edge(bindings.nextTab)) pendingNextTabAt = now;

        if (pendingNextTabAt !== null && now - pendingNextTabAt >= TAB_CHORD_WINDOW_MS) {
          pendingNextTabAt = null;
          onAction({ type: 'next-tab' });
        }

        if (edge(bindings.newTab)) onAction({ type: 'new-tab' });
      }

      previousButtons = pad.buttons.map((button) => button.pressed);
      ignoreButtonEdges = false;
    }

    frame = requestAnimationFrame(tick);
  }

  return () => {
    stopped = true;
    cancelAnimationFrame(frame);
    try {
      chrome.storage.onChanged.removeListener(onStorageChanged);
    } catch {}
  };
}
