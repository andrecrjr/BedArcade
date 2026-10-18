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
};

const BUTTON_ICONS: Record<number, ButtonIcon> = {
  0: {
    xbox: 'xbox_button_a.svg',
    playstation: 'playstation_button_cross.svg',
    xboxName: 'A',
    playstationName: 'Cross',
  },
  1: {
    xbox: 'xbox_button_b.svg',
    playstation: 'playstation_button_circle.svg',
    xboxName: 'B',
    playstationName: 'Circle',
  },
  2: {
    xbox: 'xbox_button_x.svg',
    playstation: 'playstation_button_square.svg',
    xboxName: 'X',
    playstationName: 'Square',
  },
  3: {
    xbox: 'xbox_button_y.svg',
    playstation: 'playstation_button_triangle.svg',
    xboxName: 'Y',
    playstationName: 'Triangle',
  },
  4: {
    xbox: 'xbox_lb.svg',
    playstation: 'playstation_trigger_l1.svg',
    xboxName: 'LB',
    playstationName: 'L1',
  },
  5: {
    xbox: 'xbox_rb.svg',
    playstation: 'playstation_trigger_r1.svg',
    xboxName: 'RB',
    playstationName: 'R1',
  },
  6: {
    xbox: 'xbox_lt.svg',
    playstation: 'playstation_trigger_l2.svg',
    xboxName: 'LT',
    playstationName: 'L2',
  },
  7: {
    xbox: 'xbox_rt.svg',
    playstation: 'playstation_trigger_r2.svg',
    xboxName: 'RT',
    playstationName: 'R2',
  },
  8: {
    xbox: 'xbox_button_view.svg',
    playstation: 'playstation5_button_create.svg',
    xboxName: 'View',
    playstationName: 'Create',
  },
  9: {
    xbox: 'xbox_button_menu.svg',
    playstation: 'playstation5_button_options.svg',
    xboxName: 'Menu',
    playstationName: 'Options',
  },
  10: {
    xbox: 'xbox_stick_l_press.svg',
    playstation: 'playstation_button_l3.svg',
    xboxName: 'Left stick press',
    playstationName: 'L3',
  },
  11: {
    xbox: 'xbox_stick_r_press.svg',
    playstation: 'playstation_button_r3.svg',
    xboxName: 'Right stick press',
    playstationName: 'R3',
  },
  12: {
    xbox: 'xbox_dpad_up.svg',
    playstation: 'playstation_dpad_up.svg',
    xboxName: 'D-pad up',
    playstationName: 'D-pad up',
  },
  13: {
    xbox: 'xbox_dpad_down.svg',
    playstation: 'playstation_dpad_down.svg',
    xboxName: 'D-pad down',
    playstationName: 'D-pad down',
  },
  14: {
    xbox: 'xbox_dpad_left.svg',
    playstation: 'playstation_dpad_left.svg',
    xboxName: 'D-pad left',
    playstationName: 'D-pad left',
  },
  15: {
    xbox: 'xbox_dpad_right.svg',
    playstation: 'playstation_dpad_right.svg',
    xboxName: 'D-pad right',
    playstationName: 'D-pad right',
  },
};

function itemIcon(item: GamepadPromptItem, platform: 'xbox' | 'playstation') {
  if (typeof item === 'number') {
    const button = BUTTON_ICONS[item];
    return button
      ? platform === 'xbox'
        ? { filename: button.xbox, name: button.xboxName }
        : { filename: button.playstation, name: button.playstationName }
      : null;
  }

  const name = item.stick === 'left' ? 'Left stick' : 'Right stick';
  const direction = item.direction ? ' ' + item.direction : '';
  const prefix = platform === 'xbox' ? 'xbox' : 'playstation';
  return {
    filename:
      prefix + '_stick_' + item.stick + (item.direction ? '_' + item.direction : '') + '.svg',
    name: name + direction,
  };
}

export function createGamepadPrompt(items: readonly GamepadPromptItem[], chord = false) {
  const prompt = document.createElement('kbd');
  prompt.className = 'gamepad-prompt';
  Object.assign(prompt.style, {
    display: 'inline-flex',
    alignItems: 'center',
    gap: '6px',
    border: '0',
    padding: '0',
    background: 'transparent',
    whiteSpace: 'nowrap',
  });

  for (const platform of ['xbox', 'playstation'] as const) {
    const platformLabel = platform === 'xbox' ? 'Xbox' : 'PlayStation';
    const group = document.createElement('span');
    group.className = 'gamepad-prompt-platform';
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
      const icon = itemIcon(item, platform);
      if (!icon) continue;
      const image = document.createElement('img');
      image.src = chrome.runtime.getURL('gamepad-icons/' + platform + '/' + icon.filename);
      image.alt = '';
      image.title = platformLabel + ' ' + icon.name;
      image.draggable = false;
      Object.assign(image.style, { display: 'block', width: '20px', height: '20px' });
      names.push(icon.name);
      group.append(image);
    }

    group.setAttribute('aria-label', platformLabel + ': ' + names.join(', '));
    prompt.append(group);
  }

  const xboxNames = items.flatMap((item) => {
    const icon = itemIcon(item, 'xbox');
    return icon ? [icon.name] : [];
  });
  const playstationNames = items.flatMap((item) => {
    const icon = itemIcon(item, 'playstation');
    return icon ? [icon.name] : [];
  });
  const separator = chord ? ' + ' : ', ';
  prompt.setAttribute(
    'aria-label',
    'Xbox: ' + xboxNames.join(separator) + '; PlayStation: ' + playstationNames.join(separator),
  );
  return prompt;
}
