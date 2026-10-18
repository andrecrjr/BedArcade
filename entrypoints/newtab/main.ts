import { startGamepadController, type GamepadAction, type GamepadState } from '../../lib/gamepad';
import {
  DEFAULT_GAMEPAD_BINDINGS,
  GAMEPAD_BINDINGS_STORAGE_KEY,
  GAMEPAD_SELECT_BUTTON,
  GAMEPAD_START_BUTTON,
  loadGamepadBindings,
  normalizeGamepadBindings,
  type GamepadBindings,
} from '../../lib/gamepad-bindings';
import {
  attachFloatingKeyboard,
  hideFloatingKeyboard,
  submitForm,
} from '../../lib/floating-keyboard';
import {
  createGamepadPrompt,
  getGamepadIconFamily,
  normalizeGamepadIconStyle,
  saveGamepadIconStyle,
  setActiveGamepadId,
  watchGamepadIconChanges,
  type GamepadPromptItem,
} from '../../lib/gamepad-icons';
import { nearest } from '../../lib/spatial';
import { showSelectPicker } from '../../lib/select-picker';
import { sendTabAction } from '../../lib/tab-actions';
import { normalizeUrl } from '../../lib/url';
import { createPinsFeature } from './features/pins';
import { createTvModeSettings } from './features/tv-mode-settings';
import {
  applyTranslations,
  detectAppLocale,
  loadAppLocale,
  normalizeAppLocale,
  saveAppLocale,
  translate,
  watchAppLocale,
  type AppLocale,
  type TranslationKey,
} from '../../lib/i18n';
import { completeOnboarding, loadOnboardingCompleted } from '../../lib/onboarding';
import './style.css';

function element<T extends HTMLElement>(id: string): T {
  const found = document.getElementById(id);
  if (!found) throw new Error(`Missing #${id}`);
  return found as T;
}

const pinGrid = element<HTMLDivElement>('pin-grid');
const homePanel = element<HTMLElement>('home-panel');
const urlInput = element<HTMLInputElement>('site-url');
const labelInput = element<HTMLInputElement>('site-label');
const form = element<HTMLFormElement>('site-form');
const feedback = element<HTMLParagraphElement>('feedback');
const savePinButton = element<HTMLButtonElement>('save-pin');
const editPinsButton = element<HTMLButtonElement>('edit-pins');
const controllerStatus = element<HTMLDivElement>('controller-status');
const controllerHints = element<HTMLDivElement>('controller-hints');
const tvModeSettings = element<HTMLElement>('tv-mode-settings');
const tvModeEnabled = element<HTMLInputElement>('tv-mode-enabled');
const tvModeHint = element<HTMLParagraphElement>('tv-mode-hint');
const tvUrlField = element<HTMLDivElement>('tv-url-field');
const tvUrlInput = element<HTMLInputElement>('tv-url');
const languageSelect = element<HTMLSelectElement>('language-select');
const helpLink = element<HTMLButtonElement>('help-link');
const onboardingPanel = element<HTMLElement>('onboarding-panel');
const onboardingControlsCopy = element<HTMLParagraphElement>('onboarding-controls-copy');
const onboardingCommands = element<HTMLUListElement>('onboarding-commands');
const controllerIconChoice = element<HTMLDivElement>('controller-icon-choice');
const controllerIconStyleSelect = element<HTMLSelectElement>('controller-icon-style');
const onboardingStart = element<HTMLButtonElement>('onboarding-start');
const onboardingSkip = element<HTMLButtonElement>('onboarding-skip');
const stopFloatingKeyboard = attachFloatingKeyboard({ urlShortcuts: true });

let appLocale: AppLocale = detectAppLocale();
let currentBindings = DEFAULT_GAMEPAD_BINDINGS;
let currentControllerState: GamepadState = {
  connected: false,
  enabled: true,
  gamepadId: null,
};
let activeSelectPicker: HTMLDialogElement | null = null;
let feedbackKey: TranslationKey | null = null;
let feedbackValues: Record<string, string | number> = {};
let onboardingVisible = true;
let languageChanged = false;
const currentTabId = chrome.tabs
  .getCurrent()
  .then((tab) => tab?.id)
  .catch(() => undefined);

function setFeedback(
  key: TranslationKey,
  values: Record<string, string | number> = {},
  isError = false,
) {
  feedbackKey = key;
  feedbackValues = values;
  feedback.textContent = translate(appLocale, key, values);
  feedback.dataset.error = String(isError);
}

const tvMode = createTvModeSettings({
  urlInput,
  settings: tvModeSettings,
  enabled: tvModeEnabled,
  hint: tvModeHint,
  urlField: tvUrlField,
  tvUrlInput,
  getLocale: () => appLocale,
  setFeedback,
});
const pins = createPinsFeature({
  pinGrid,
  form,
  urlInput,
  labelInput,
  savePinButton,
  editPinsButton,
  getLocale: () => appLocale,
  canFocusFirstPin: () => !onboardingVisible,
  setFeedback,
  onUrlChange: () => void tvMode.sync(true),
});

function renderControllerStatus() {
  const { connected, enabled } = currentControllerState;
  controllerStatus.textContent = !connected
    ? translate(appLocale, 'controllerConnect')
    : !enabled
      ? translate(appLocale, 'controllerPaused')
      : translate(appLocale, 'controllerReadyDetails');
}

function controllerCommands(bindings: GamepadBindings) {
  return [
    {
      id: 'move',
      buttons: [
        bindings.moveUp,
        bindings.moveDown,
        bindings.moveLeft,
        bindings.moveRight,
        { stick: 'left' },
      ],
      label: 'hintMove',
    },
    {
      id: 'scroll',
      buttons: [
        { stick: 'right', direction: 'up' },
        { stick: 'right', direction: 'down' },
      ],
      label: 'hintScroll',
    },
    { id: 'activate', buttons: [bindings.activate], label: 'hintSelect' },
    { id: 'submit', buttons: [GAMEPAD_START_BUTTON], label: 'hintSubmit' },
    { id: 'back', buttons: [bindings.back], label: 'hintBack' },
    {
      id: 'previous-tab',
      buttons: [bindings.previousTab],
      label: 'hintPreviousTab',
    },
    { id: 'next-tab', buttons: [bindings.nextTab], label: 'hintNextTab' },
    {
      id: 'reload',
      buttons: [bindings.refreshModifier, bindings.refreshTrigger],
      chord: true,
      label: 'hintRefresh',
    },
    { id: 'new-tab', buttons: [bindings.newTab], label: 'hintNewTab' },
    { id: 'pause', buttons: [GAMEPAD_SELECT_BUTTON], label: 'hintPause' },
  ] satisfies {
    id: string;
    buttons: GamepadPromptItem[];
    chord?: boolean;
    label: TranslationKey;
  }[];
}

function renderOnboardingControls() {
  onboardingControlsCopy.textContent = translate(appLocale, 'onboardingControlsCopy');
  onboardingCommands.replaceChildren(
    ...controllerCommands(currentBindings).map(({ id, buttons, chord, label }) => {
      const row = document.createElement('li');
      row.dataset.onboardingCommand = id;
      const key = createGamepadPrompt(buttons, chord, appLocale);
      const description = document.createElement('span');
      description.dataset.commandLabel = label;
      description.textContent = translate(appLocale, label);
      row.append(key, description);
      return row;
    }),
  );
}

function renderControllerHints(bindings: GamepadBindings) {
  controllerHints.setAttribute('aria-label', translate(appLocale, 'controllerHints'));
  controllerHints.replaceChildren(
    ...controllerCommands(bindings).map(({ buttons, chord, label }) => {
      const hint = document.createElement('span');
      const keyElement = createGamepadPrompt(buttons, chord, appLocale);
      hint.append(keyElement, document.createTextNode(translate(appLocale, label)));
      return hint;
    }),
  );
}

function renderGamepadIconChoice(style: 'auto' | 'xbox' | 'playstation') {
  controllerIconStyleSelect.value = style;
  controllerIconChoice.hidden = style === 'auto' && getGamepadIconFamily() !== 'generic';
}

function setLocale(locale: AppLocale) {
  appLocale = locale;
  languageSelect.value = locale;
  applyTranslations(document, locale);
  renderControllerStatus();
  renderControllerHints(currentBindings);
  renderOnboardingControls();
  pins.render();
  tvMode.render();
  if (feedbackKey) feedback.textContent = translate(locale, feedbackKey, feedbackValues);
}

applyTranslations(document, appLocale);
renderControllerHints(currentBindings);
renderOnboardingControls();
tvMode.render();
const stopGamepadIconWatch = watchGamepadIconChanges((style) => {
  renderGamepadIconChoice(style);
  renderControllerHints(currentBindings);
  renderOnboardingControls();
});
const stopLocaleWatch = watchAppLocale((locale) => {
  languageChanged = true;
  setLocale(locale);
});
void loadAppLocale().then((locale) => {
  if (!languageChanged) setLocale(locale);
});

function setOnboardingVisible(visible: boolean) {
  onboardingVisible = visible;
  onboardingPanel.hidden = !visible;
  homePanel.inert = visible;
  if (visible) onboardingStart.focus({ preventScroll: true });
  else {
    const firstPin = pinGrid.querySelector<HTMLButtonElement>('[data-launch]');
    (firstPin ?? urlInput).focus({ preventScroll: true });
  }
}

function finishOnboarding() {
  void completeOnboarding().then(() => setOnboardingVisible(false));
}

void loadOnboardingCompleted().then((completed) => {
  setOnboardingVisible(!completed);
  document.documentElement.dataset.onboardingReady = 'true';
});
languageSelect.addEventListener('change', () => {
  const locale = normalizeAppLocale(languageSelect.value);
  if (!locale) return;
  languageChanged = true;
  setLocale(locale);
  void saveAppLocale(locale).catch(() => setFeedback('feedbackCouldNotSaveLanguage', {}, true));
});
controllerIconStyleSelect.addEventListener('change', () => {
  const style = normalizeGamepadIconStyle(controllerIconStyleSelect.value);
  void saveGamepadIconStyle(style).catch(() => setFeedback('feedbackCouldNotSaveIconStyle'));
});
helpLink.addEventListener('click', () => setOnboardingVisible(true));
onboardingStart.addEventListener('click', finishOnboarding);
onboardingSkip.addEventListener('click', finishOnboarding);

let controllerHintsChanged = false;
const onGamepadBindingsChanged = (
  changes: { [key: string]: chrome.storage.StorageChange },
  areaName: string,
) => {
  const change = changes[GAMEPAD_BINDINGS_STORAGE_KEY];
  if (areaName !== 'local' || !change) return;
  controllerHintsChanged = true;
  currentBindings = normalizeGamepadBindings(change.newValue);
  renderControllerHints(currentBindings);
  renderOnboardingControls();
};
try {
  chrome.storage.onChanged.addListener(onGamepadBindingsChanged);
} catch {}
void loadGamepadBindings()
  .then((bindings) => {
    if (!controllerHintsChanged) {
      currentBindings = bindings;
      renderControllerHints(bindings);
      renderOnboardingControls();
    }
  })
  .catch(() => {});

async function savePin() {
  const site = pins.readDraft();
  if (!site) return;

  const tvModeResult = await tvMode.saveForUrl(site.url);
  if (tvModeResult === 'failed') return;

  const saved = await pins.save(site);
  if (!saved && tvModeResult === 'saved')
    setFeedback('feedbackCouldNotSaveShortcutTvModeSaved', { label: site.label }, true);
}

async function goToAddress() {
  const url = normalizeUrl(urlInput.value);
  if (!url) {
    setFeedback('feedbackInvalidWebsite', {}, true);
    urlInput.focus();
    return;
  }
  if ((await tvMode.saveForUrl(url)) === 'failed') return;
  window.location.assign(url);
}

function activateFocusedElement() {
  const active = document.activeElement;
  if (!(active instanceof HTMLElement) || active === document.body) return;
  active.focus({ preventScroll: true });
  active.click();
  if (active instanceof HTMLSelectElement && active.isConnected) {
    const picker = showSelectPicker(active, appLocale);
    if (!picker) return;
    activeSelectPicker = picker.dialog;
    picker.dialog.addEventListener(
      'close',
      () => {
        if (activeSelectPicker === picker.dialog) activeSelectPicker = null;
        if (active.isConnected) active.focus({ preventScroll: true });
      },
      { once: true },
    );
  }
}

function moveFocus(direction: 'up' | 'down' | 'left' | 'right') {
  const active = document.activeElement instanceof HTMLElement ? document.activeElement : null;
  const keyboard = document.querySelector<HTMLElement>('[data-gamepad-navigator="keyboard"]');
  if (
    direction === 'down' &&
    keyboard &&
    keyboard.style.display !== 'none' &&
    !keyboard.contains(active)
  ) {
    const firstKey = keyboard.querySelector<HTMLButtonElement>('[data-key="q"]');
    if (firstKey) {
      firstKey.focus({ preventScroll: true });
      return;
    }
  }

  const next = nearest(active, direction);
  if (next) {
    next.focus({ preventScroll: true });
    const rect = next.getBoundingClientRect();
    if (rect.bottom > innerHeight || rect.top < 0) next.scrollIntoView({ block: 'nearest' });
  } else {
    window.scrollBy({ top: direction === 'down' ? 220 : direction === 'up' ? -220 : 0 });
  }
}

function handleGamepadAction(action: GamepadAction) {
  if (action.type === 'move') moveFocus(action.direction);
  else if (action.type === 'activate') activateFocusedElement();
  else if (action.type === 'submit')
    submitForm(document.activeElement instanceof HTMLElement ? document.activeElement : null);
  else if (action.type === 'scroll') window.scrollBy(0, action.amount);
  else if (action.type === 'back') {
    if (activeSelectPicker?.open) activeSelectPicker.close('cancel');
    else if (hideFloatingKeyboard()) return;
    else void currentTabId.then((id) => sendTabAction('back', id));
  } else if (action.type === 'refresh') window.location.reload();
  else if (action.type === 'previous-tab')
    void currentTabId.then((id) => sendTabAction('prev-tab', id));
  else if (action.type === 'next-tab')
    void currentTabId.then((id) => sendTabAction('next-tab', id));
  else if (action.type === 'new-tab') void currentTabId.then((id) => sendTabAction('new-tab', id));
}

function handleGamepadState(state: GamepadState) {
  setActiveGamepadId(state.gamepadId);
  currentControllerState = state;
  controllerStatus.dataset.connected = String(state.connected);
  controllerStatus.dataset.enabled = String(state.enabled);
  renderControllerStatus();
}

editPinsButton.addEventListener('click', () => {
  pins.toggleEditing();
});

form.addEventListener('submit', (event) => {
  event.preventDefault();
  void goToAddress();
});

urlInput.addEventListener('input', () => void tvMode.sync());
tvModeEnabled.addEventListener('change', tvMode.render);
savePinButton.addEventListener('click', () => void savePin());

void pins.load();

const stopGamepad = startGamepadController({
  onAction: handleGamepadAction,
  onStateChange: handleGamepadState,
});

window.addEventListener(
  'pagehide',
  () => {
    stopGamepad();
    stopFloatingKeyboard();
    try {
      chrome.storage.onChanged.removeListener(onGamepadBindingsChanged);
    } catch {}
    stopGamepadIconWatch();
    stopLocaleWatch();
  },
  { once: true },
);
