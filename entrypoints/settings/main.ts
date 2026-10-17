import {
  DEFAULT_GAMEPAD_BINDINGS,
  GAMEPAD_BINDING_NAMES,
  GAMEPAD_BUTTON_OPTIONS,
  loadGamepadBindings,
  normalizeGamepadBindings,
  saveGamepadBindings,
} from '../../lib/gamepad-bindings';
import '../newtab/style.css';
import './style.css';

const form = document.querySelector<HTMLFormElement>('#settings-form');
const feedback = document.querySelector<HTMLParagraphElement>('#settings-feedback');
const restoreButton = document.querySelector<HTMLButtonElement>('#restore-defaults');
const selects = [...document.querySelectorAll<HTMLSelectElement>('[data-binding]')];

if (!form || !feedback || !restoreButton) throw new Error('Controller settings page is incomplete');
const settingsFeedback = feedback;

function setFeedback(message: string, isError = false) {
  settingsFeedback.textContent = message;
  settingsFeedback.dataset.error = String(isError);
}

function displayBindings(bindings: typeof DEFAULT_GAMEPAD_BINDINGS) {
  for (const select of selects) {
    const name = select.dataset.binding;
    if (!name || !GAMEPAD_BINDING_NAMES.includes(name as (typeof GAMEPAD_BINDING_NAMES)[number]))
      continue;

    select.replaceChildren(
      ...GAMEPAD_BUTTON_OPTIONS.map((button) => {
        const option = document.createElement('option');
        option.value = String(button.value);
        option.textContent = `${button.value} · ${button.label}`;
        return option;
      }),
    );
    select.value = String(bindings[name as keyof typeof DEFAULT_GAMEPAD_BINDINGS]);
  }
}

function readBindings() {
  const values: Record<string, number> = {};
  for (const select of selects) {
    const name = select.dataset.binding;
    if (name) values[name] = Number(select.value);
  }
  return normalizeGamepadBindings(values);
}

async function save(bindings: typeof DEFAULT_GAMEPAD_BINDINGS, message: string) {
  try {
    await saveGamepadBindings(bindings);
    setFeedback(message);
  } catch {
    setFeedback('Could not save controller settings.', true);
  }
}

void loadGamepadBindings()
  .then(displayBindings)
  .catch(() => {
    displayBindings(DEFAULT_GAMEPAD_BINDINGS);
    setFeedback('Could not load saved controls. Defaults are shown.', true);
  });

form.addEventListener('submit', (event) => {
  event.preventDefault();
  void save(readBindings(), 'Controls saved.');
});

restoreButton.addEventListener('click', () => {
  displayBindings(DEFAULT_GAMEPAD_BINDINGS);
  void save(DEFAULT_GAMEPAD_BINDINGS, 'Default controls restored.');
});
