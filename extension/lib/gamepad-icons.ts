import { detectAppLocale, translate, type AppLocale, type TranslationKey } from './i18n';

export type GamepadPromptItem =
  | number
  | {
      stick: 'left' | 'right';
      direction?: 'up' | 'down' | 'left' | 'right';
    };

type ButtonIcon = {
  xbox: string;
  playstation: string;
  xboxName: string;
  playstationName: string;
  genericName: TranslationKey;
};

export type GamepadIconStyle = 'auto' | 'xbox' | 'playstation';
export type GamepadIconFamily = Exclude<GamepadIconStyle, 'auto'> | 'generic';

export const GAMEPAD_ICON_STYLE_STORAGE_KEY = 'gamepadIconStyle';

let iconStyle: GamepadIconStyle = 'auto';
let activeGamepadId: string | null | undefined;
let storageListenerAttached = false;
let iconStyleRevision = 0;
const iconChangeListeners = new Set<(style: GamepadIconStyle) => void>();

export function normalizeGamepadIconStyle(value: unknown): GamepadIconStyle {
  return value === 'xbox' || value === 'playstation' ? value : 'auto';
}

export function detectGamepadIconFamily(id: string | null | undefined): GamepadIconFamily | null {
  const normalized = id?.toLowerCase() ?? '';
  if (/playstation|dualshock|dualsense|sony/.test(normalized)) return 'playstation';
  if (/xbox|microsoft/.test(normalized)) return 'xbox';
  return null;
}

export function getGamepadIconFamily(): GamepadIconFamily {
  if (iconStyle !== 'auto') return iconStyle;
  const id = activeGamepadId === undefined ? getConnectedGamepadId() : activeGamepadId;
  return detectGamepadIconFamily(id) ?? 'generic';
}

export function setActiveGamepadId(id: string | null) {
  if (activeGamepadId === id) return;
  activeGamepadId = id;
  notifyIconChange();
}

async function loadGamepadIconStyle() {
  const stored = await chrome.storage.local.get(GAMEPAD_ICON_STYLE_STORAGE_KEY);
  return normalizeGamepadIconStyle(stored[GAMEPAD_ICON_STYLE_STORAGE_KEY]);
}

export async function saveGamepadIconStyle(style: GamepadIconStyle) {
  iconStyleRevision += 1;
  iconStyle = style;
  notifyIconChange();
  await chrome.storage.local.set({ [GAMEPAD_ICON_STYLE_STORAGE_KEY]: style });
}

export function watchGamepadIconChanges(listener: (style: GamepadIconStyle) => void) {
  iconChangeListeners.add(listener);
  listener(iconStyle);
  if (!storageListenerAttached) {
    try {
      chrome.storage.onChanged.addListener(onStorageChanged);
      storageListenerAttached = true;
    } catch {}
    const revision = iconStyleRevision;
    void loadGamepadIconStyle()
      .then((style) => {
        if (revision !== iconStyleRevision) return;
        iconStyle = style;
        notifyIconChange();
      })
      .catch(() => {});
  }

  return () => {
    iconChangeListeners.delete(listener);
    if (iconChangeListeners.size || !storageListenerAttached) return;
    try {
      chrome.storage.onChanged.removeListener(onStorageChanged);
      storageListenerAttached = false;
    } catch {}
  };
}

function onStorageChanged(
  changes: { [key: string]: chrome.storage.StorageChange },
  areaName: string,
) {
  const change = changes[GAMEPAD_ICON_STYLE_STORAGE_KEY];
  if (areaName !== 'local' || !change) return;
  iconStyleRevision += 1;
  iconStyle = normalizeGamepadIconStyle(change.newValue);
  notifyIconChange();
}

function notifyIconChange() {
  for (const listener of iconChangeListeners) listener(iconStyle);
}

function getConnectedGamepadId() {
  try {
    return navigator.getGamepads().find((pad) => pad?.connected)?.id ?? null;
  } catch {
    return null;
  }
}

const BUTTON_ICONS: Record<number, ButtonIcon> = {
  0: {
    xbox: 'xbox_button_a.svg',
    playstation: 'playstation_button_cross.svg',
    xboxName: 'A',
    playstationName: 'Cross',
    genericName: 'genericButtonBottom',
  },
  1: {
    xbox: 'xbox_button_b.svg',
    playstation: 'playstation_button_circle.svg',
    xboxName: 'B',
    playstationName: 'Circle',
    genericName: 'genericButtonRight',
  },
  2: {
    xbox: 'xbox_button_x.svg',
    playstation: 'playstation_button_square.svg',
    xboxName: 'X',
    playstationName: 'Square',
    genericName: 'genericButtonLeft',
  },
  3: {
    xbox: 'xbox_button_y.svg',
    playstation: 'playstation_button_triangle.svg',
    xboxName: 'Y',
    playstationName: 'Triangle',
    genericName: 'genericButtonTop',
  },
  4: {
    xbox: 'xbox_lb.svg',
    playstation: 'playstation_trigger_l1.svg',
    xboxName: 'LB',
    playstationName: 'L1',
    genericName: 'genericLeftShoulder',
  },
  5: {
    xbox: 'xbox_rb.svg',
    playstation: 'playstation_trigger_r1.svg',
    xboxName: 'RB',
    playstationName: 'R1',
    genericName: 'genericRightShoulder',
  },
  6: {
    xbox: 'xbox_lt.svg',
    playstation: 'playstation_trigger_l2.svg',
    xboxName: 'LT',
    playstationName: 'L2',
    genericName: 'genericLeftTrigger',
  },
  7: {
    xbox: 'xbox_rt.svg',
    playstation: 'playstation_trigger_r2.svg',
    xboxName: 'RT',
    playstationName: 'R2',
    genericName: 'genericRightTrigger',
  },
  8: {
    xbox: 'xbox_button_view.svg',
    playstation: 'playstation5_button_create.svg',
    xboxName: 'View',
    playstationName: 'Create',
    genericName: 'genericCenterLeft',
  },
  9: {
    xbox: 'xbox_button_menu.svg',
    playstation: 'playstation5_button_options.svg',
    xboxName: 'Menu',
    playstationName: 'Options',
    genericName: 'genericCenterRight',
  },
  10: {
    xbox: 'xbox_stick_l_press.svg',
    playstation: 'playstation_button_l3.svg',
    xboxName: 'Left stick press',
    playstationName: 'L3',
    genericName: 'genericLeftStickPress',
  },
  11: {
    xbox: 'xbox_stick_r_press.svg',
    playstation: 'playstation_button_r3.svg',
    xboxName: 'Right stick press',
    playstationName: 'R3',
    genericName: 'genericRightStickPress',
  },
  12: {
    xbox: 'xbox_dpad_up.svg',
    playstation: 'playstation_dpad_up.svg',
    xboxName: 'D-pad up',
    playstationName: 'D-pad up',
    genericName: 'buttonDpadUp',
  },
  13: {
    xbox: 'xbox_dpad_down.svg',
    playstation: 'playstation_dpad_down.svg',
    xboxName: 'D-pad down',
    playstationName: 'D-pad down',
    genericName: 'buttonDpadDown',
  },
  14: {
    xbox: 'xbox_dpad_left.svg',
    playstation: 'playstation_dpad_left.svg',
    xboxName: 'D-pad left',
    playstationName: 'D-pad left',
    genericName: 'buttonDpadLeft',
  },
  15: {
    xbox: 'xbox_dpad_right.svg',
    playstation: 'playstation_dpad_right.svg',
    xboxName: 'D-pad right',
    playstationName: 'D-pad right',
    genericName: 'buttonDpadRight',
  },
};

type PromptIcon = { filename: string | null; name: string; label: string };

function itemIcon(
  item: GamepadPromptItem,
  platform: GamepadIconFamily,
  locale: AppLocale,
): PromptIcon | null {
  if (typeof item === 'number') {
    const button = BUTTON_ICONS[item];
    if (platform === 'generic') {
      const name = button
        ? translate(locale, button.genericName)
        : translate(locale, 'buttonFallback', { button: item });
      return { filename: null, name, label: name };
    }
    if (!button) return null;
    const name = platform === 'xbox' ? button.xboxName : button.playstationName;
    return {
      filename: platform === 'xbox' ? button.xbox : button.playstation,
      name,
      label: name,
    };
  }

  if (platform === 'generic') {
    const name = translate(
      locale,
      item.stick === 'left' ? 'genericLeftStick' : 'genericRightStick',
    );
    const direction = item.direction ? ` ${directionArrow(item.direction)}` : '';
    return { filename: null, name: name + direction, label: name + direction };
  }
  const name = item.stick === 'left' ? 'Left stick' : 'Right stick';
  const direction = item.direction ? ' ' + item.direction : '';
  const prefix = platform === 'xbox' ? 'xbox' : 'playstation';
  const stickName = item.stick === 'left' ? 'l' : 'r';
  const directionSuffix = item.direction ? `_${item.direction}` : '';
  return {
    filename: `${prefix}_stick_${stickName}${directionSuffix}.svg`,
    name: name + direction,
    label: name + direction,
  };
}

function directionArrow(direction: 'up' | 'down' | 'left' | 'right') {
  return { up: '↑', down: '↓', left: '←', right: '→' }[direction];
}

export function createGamepadPrompt(
  items: readonly GamepadPromptItem[],
  chord = false,
  locale: AppLocale = detectAppLocale(),
) {
  const prompt = document.createElement('kbd');
  prompt.className = 'gamepad-prompt';
  const family = getGamepadIconFamily();
  prompt.dataset.gamepadFamily = family;
  Object.assign(prompt.style, {
    display: 'inline-flex',
    alignItems: 'center',
    gap: '6px',
    border: '0',
    padding: '0',
    background: 'transparent',
    whiteSpace: 'nowrap',
  });

  const platformLabel =
    family === 'generic'
      ? translate(locale, 'genericButtonPositions')
      : family === 'xbox'
        ? 'Xbox'
        : 'PlayStation';
  const group = document.createElement('span');
  group.className = `gamepad-prompt-platform gamepad-prompt-${family}`;
  Object.assign(group.style, {
    display: 'inline-flex',
    alignItems: 'center',
    gap: '4px',
    border: '1px solid rgba(235,235,235,.24)',
    borderRadius: '999px',
    padding: '2px 6px',
    background: 'rgba(230,230,230,.08)',
  });

  const brand = document.createElement('span');
  brand.className = 'gamepad-prompt-brand';
  brand.textContent = platformLabel;
  brand.setAttribute('aria-hidden', 'true');
  Object.assign(brand.style, {
    color: '#f1f1f1',
    font: '700 10px system-ui,sans-serif',
  });
  group.append(brand);

  const names: string[] = [];
  for (const [index, item] of items.entries()) {
    if (chord && index > 0) {
      const plus = document.createElement('span');
      plus.textContent = '+';
      plus.setAttribute('aria-hidden', 'true');
      group.append(plus);
    }
    const icon = itemIcon(item, family, locale);
    if (!icon) continue;
    if (icon.filename) {
      const image = document.createElement('img');
      image.src = chrome.runtime.getURL('gamepad-icons/' + family + '/' + icon.filename);
      image.alt = '';
      image.title = platformLabel + ' ' + icon.name;
      image.draggable = false;
      Object.assign(image.style, { display: 'block', width: '20px', height: '20px' });
      group.append(image);
    } else {
      const key = document.createElement('span');
      key.className = 'gamepad-prompt-neutral-key';
      key.textContent = icon.label;
      key.title = icon.name;
      Object.assign(key.style, {
        border: '1px solid rgba(235,235,235,.24)',
        borderRadius: '5px',
        padding: '3px 5px',
        color: '#f1f1f1',
        font: '600 10px system-ui,sans-serif',
      });
      group.append(key);
    }
    names.push(icon.name);
  }

  group.setAttribute('aria-label', platformLabel + ': ' + names.join(chord ? ' + ' : ', '));
  prompt.setAttribute('aria-label', group.getAttribute('aria-label') ?? platformLabel);
  prompt.append(group);
  return prompt;
}
