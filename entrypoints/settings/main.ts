import {
  DEFAULT_GAMEPAD_BINDINGS,
  GAMEPAD_BINDING_NAMES,
  GAMEPAD_BINDABLE_BUTTON_OPTIONS,
  loadGamepadBindings,
  normalizeGamepadBindings,
  saveGamepadBindings,
} from '../../lib/gamepad-bindings';
import {
  applyTranslations,
  detectAppLocale,
  gamepadButtonLabel,
  loadAppLocale,
  normalizeAppLocale,
  saveAppLocale,
  translate,
  watchAppLocale,
  type AppLocale,
  type TranslationKey,
} from '../../lib/i18n';
import { createGamepadPrompt, watchGamepadIconChanges } from '../../lib/gamepad-icons';
import '../newtab/style.css';
import './style.css';

const form = document.querySelector<HTMLFormElement>('#settings-form');
const feedback = document.querySelector<HTMLParagraphElement>('#settings-feedback');
const restoreButton = document.querySelector<HTMLButtonElement>('#restore-defaults');
const selects = [...document.querySelectorAll<HTMLSelectElement>('[data-binding]')];
const fixedPrompts = [...document.querySelectorAll<HTMLElement>('[data-fixed-prompt]')];

if (!form || !feedback || !restoreButton) throw new Error('Controller settings page is incomplete');
const settingsFeedback = feedback;
const languageSelect = document.querySelector<HTMLSelectElement>('#language-select');
if (!languageSelect) throw new Error('The language selector is missing');
const languagePicker = languageSelect;

let appLocale: AppLocale = detectAppLocale();
let feedbackKey: TranslationKey | null = null;
let feedbackIsError = false;
let languageChanged = false;

function setFeedback(key: TranslationKey, isError = false) {
  feedbackKey = key;
  feedbackIsError = isError;
  settingsFeedback.textContent = translate(appLocale, key);
  settingsFeedback.dataset.error = String(isError);
}

function displayBindings(bindings: typeof DEFAULT_GAMEPAD_BINDINGS) {
  for (const select of selects) {
    const name = select.dataset.binding;
    if (!name || !GAMEPAD_BINDING_NAMES.includes(name as (typeof GAMEPAD_BINDING_NAMES)[number]))
      continue;

    select.replaceChildren(
      ...GAMEPAD_BINDABLE_BUTTON_OPTIONS.map((button) => {
        const option = document.createElement('option');
        option.value = String(button.value);
        option.textContent = `${button.value} · ${gamepadButtonLabel(appLocale, button.value)}`;
        return option;
      }),
    );
    select.value = String(bindings[name as keyof typeof DEFAULT_GAMEPAD_BINDINGS]);
  }
}

function displayFixedPrompts() {
  for (const prompt of fixedPrompts) {
    const button = Number(prompt.dataset.fixedPrompt);
    prompt.replaceChildren(createGamepadPrompt([button], false, appLocale));
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

async function save(bindings: typeof DEFAULT_GAMEPAD_BINDINGS, message: TranslationKey) {
  try {
    await saveGamepadBindings(bindings);
    setFeedback(message);
  } catch {
    setFeedback('feedbackCouldNotSaveControls', true);
  }
}

function setLocale(locale: AppLocale) {
  appLocale = locale;
  languagePicker.value = locale;
  applyTranslations(document, locale);
  displayBindings(readBindings());
  if (feedbackKey) {
    settingsFeedback.textContent = translate(locale, feedbackKey);
    settingsFeedback.dataset.error = String(feedbackIsError);
  }
}

applyTranslations(document, appLocale);
displayFixedPrompts();
const stopGamepadIconWatch = watchGamepadIconChanges(displayFixedPrompts);
const stopLocaleWatch = watchAppLocale((locale) => {
  languageChanged = true;
  setLocale(locale);
});
void loadAppLocale().then((locale) => {
  if (!languageChanged) setLocale(locale);
});
languagePicker.addEventListener('change', () => {
  const locale = normalizeAppLocale(languagePicker.value);
  if (!locale) return;
  languageChanged = true;
  setLocale(locale);
  void saveAppLocale(locale).catch(() => setFeedback('feedbackCouldNotSaveLanguage', true));
});

void loadGamepadBindings()
  .then(displayBindings)
  .catch(() => {
    displayBindings(DEFAULT_GAMEPAD_BINDINGS);
    setFeedback('feedbackCouldNotLoadControls', true);
  });

form.addEventListener('submit', (event) => {
  event.preventDefault();
  void save(readBindings(), 'controlsSaved');
});

restoreButton.addEventListener('click', () => {
  displayBindings(DEFAULT_GAMEPAD_BINDINGS);
  void save(DEFAULT_GAMEPAD_BINDINGS, 'defaultsRestored');
});

window.addEventListener(
  'pagehide',
  () => {
    stopGamepadIconWatch();
    stopLocaleWatch();
  },
  { once: true },
);
