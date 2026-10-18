export const GAMEPAD_BINDINGS_STORAGE_KEY = 'gamepadBindings';
export const GAMEPAD_SELECT_BUTTON = 8;

export const GAMEPAD_BINDING_NAMES = [
  'moveUp',
  'moveDown',
  'moveLeft',
  'moveRight',
  'activate',
  'back',
  'refreshModifier',
  'refreshTrigger',
  'previousTab',
  'nextTab',
  'newTab',
] as const;

export type GamepadBindingName = (typeof GAMEPAD_BINDING_NAMES)[number];
export type GamepadBindings = Record<GamepadBindingName, number> & {
  pause: typeof GAMEPAD_SELECT_BUTTON;
};

export const DEFAULT_GAMEPAD_BINDINGS: GamepadBindings = {
  moveUp: 12,
  moveDown: 13,
  moveLeft: 14,
  moveRight: 15,
  activate: 0,
  back: 1,
  refreshModifier: 5,
  refreshTrigger: 2,
  previousTab: 4,
  nextTab: 5,
  newTab: 11,
  pause: GAMEPAD_SELECT_BUTTON,
};

export const GAMEPAD_BUTTON_OPTIONS = [
  { value: 0, label: 'A / South', short: 'A' },
  { value: 1, label: 'B / East', short: 'B' },
  { value: 2, label: 'X / West', short: 'X' },
  { value: 3, label: 'Y / North', short: 'Y' },
  { value: 4, label: 'LB / L1', short: 'LB' },
  { value: 5, label: 'RB / R1', short: 'RB' },
  { value: 6, label: 'LT / L2', short: 'LT' },
  { value: 7, label: 'RT / R2', short: 'RT' },
  { value: 8, label: 'Select / View', short: 'SELECT' },
  { value: 9, label: 'Start / Menu', short: 'START' },
  { value: 10, label: 'L3 / Left stick', short: 'L3' },
  { value: 11, label: 'R3 / Right stick', short: 'R3' },
  { value: 12, label: 'D-pad up', short: '↑' },
  { value: 13, label: 'D-pad down', short: '↓' },
  { value: 14, label: 'D-pad left', short: '←' },
  { value: 15, label: 'D-pad right', short: '→' },
] as const;

export const GAMEPAD_BINDABLE_BUTTON_OPTIONS = GAMEPAD_BUTTON_OPTIONS.filter(
  ({ value }) => value !== GAMEPAD_SELECT_BUTTON,
);

export function normalizeGamepadBindings(value: unknown): GamepadBindings {
  const stored =
    typeof value === 'object' && value !== null ? (value as Record<string, unknown>) : {};
  const bindings = Object.fromEntries(
    GAMEPAD_BINDING_NAMES.map((name) => {
      const button = stored[name];
      return [
        name,
        typeof button === 'number' &&
        Number.isInteger(button) &&
        button >= 0 &&
        button <= 15 &&
        button !== GAMEPAD_SELECT_BUTTON
          ? button
          : DEFAULT_GAMEPAD_BINDINGS[name],
      ];
    }),
  ) as Record<GamepadBindingName, number>;
  return { ...bindings, pause: GAMEPAD_SELECT_BUTTON };
}

export function gamepadButtonName(button: number) {
  return (
    GAMEPAD_BUTTON_OPTIONS.find((option) => option.value === button)?.short ?? `Button ${button}`
  );
}

export async function loadGamepadBindings() {
  const stored = await chrome.storage.local.get(GAMEPAD_BINDINGS_STORAGE_KEY);
  return normalizeGamepadBindings(stored[GAMEPAD_BINDINGS_STORAGE_KEY]);
}

export async function saveGamepadBindings(bindings: GamepadBindings) {
  await chrome.storage.local.set({
    [GAMEPAD_BINDINGS_STORAGE_KEY]: normalizeGamepadBindings(bindings),
  });
}
