import { startGamepadController, type GamepadAction } from '../../lib/gamepad';
import {
  DEFAULT_GAMEPAD_BINDINGS,
  GAMEPAD_BINDINGS_STORAGE_KEY,
  gamepadButtonName,
  loadGamepadBindings,
  normalizeGamepadBindings,
  type GamepadBindings,
} from '../../lib/gamepad-bindings';
import { loadPinnedSites, savePinnedSites, type PinnedSite } from '../../lib/pins';
import { attachFloatingKeyboard } from '../../lib/floating-keyboard';
import { nearest } from '../../lib/spatial';
import { showSelectPicker } from '../../lib/select-picker';
import { sendTabAction } from '../../lib/tab-actions';
import {
  createDefaultTvModeProfile,
  getBuiltInTvUrl,
  loadTvModeProfile,
  normalizeTvUrl,
  type TvModeProfile,
} from '../../lib/tv-mode';
import { normalizeUrl } from '../../lib/url';
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
const onboardingStart = element<HTMLButtonElement>('onboarding-start');
const onboardingSkip = element<HTMLButtonElement>('onboarding-skip');
const stopFloatingKeyboard = attachFloatingKeyboard({ urlShortcuts: true });

let appLocale: AppLocale = detectAppLocale();
let pinnedSites: PinnedSite[] = [];
let currentBindings = DEFAULT_GAMEPAD_BINDINGS;
let currentControllerState = { connected: false, enabled: true };
let activeSelectPicker: HTMLDialogElement | null = null;
let editingPins = false;
let editingPinId: string | null = null;
let currentTvHostname: string | null = null;
let currentTvProfile: TvModeProfile | null = null;
let tvProfileLoadSequence = 0;
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

function renderControllerStatus() {
  const { connected, enabled } = currentControllerState;
  controllerStatus.textContent = !connected
    ? translate(appLocale, 'controllerConnect')
    : !enabled
      ? translate(appLocale, 'controllerPaused', {
          button: gamepadButtonName(currentBindings.pause),
        })
      : translate(appLocale, 'controllerReadyDetails');
}

function controllerCommands(bindings: GamepadBindings) {
  const directions = [bindings.moveUp, bindings.moveDown, bindings.moveLeft, bindings.moveRight]
    .map(gamepadButtonName)
    .join('/');

  return [
    { id: 'move', buttons: `${directions} / LS`, label: 'hintMove' },
    { id: 'scroll', buttons: 'RS ↑/↓', label: 'hintScroll' },
    { id: 'activate', buttons: gamepadButtonName(bindings.activate), label: 'hintSelect' },
    { id: 'back', buttons: gamepadButtonName(bindings.back), label: 'hintBack' },
    {
      id: 'previous-tab',
      buttons: gamepadButtonName(bindings.previousTab),
      label: 'hintPreviousTab',
    },
    { id: 'next-tab', buttons: gamepadButtonName(bindings.nextTab), label: 'hintNextTab' },
    {
      id: 'reload',
      buttons: `${gamepadButtonName(bindings.refreshModifier)} + ${gamepadButtonName(bindings.refreshTrigger)}`,
      label: 'hintRefresh',
    },
    { id: 'new-tab', buttons: gamepadButtonName(bindings.newTab), label: 'hintNewTab' },
    { id: 'pause', buttons: gamepadButtonName(bindings.pause), label: 'hintPause' },
  ] satisfies { id: string; buttons: string; label: TranslationKey }[];
}

function renderOnboardingControls() {
  onboardingControlsCopy.textContent = translate(appLocale, 'onboardingControlsCopy', {
    nextTab: gamepadButtonName(currentBindings.nextTab),
    refresh: `${gamepadButtonName(currentBindings.refreshModifier)} + ${gamepadButtonName(currentBindings.refreshTrigger)}`,
  });
  onboardingCommands.replaceChildren(
    ...controllerCommands(currentBindings).map(({ id, buttons, label }) => {
      const row = document.createElement('li');
      row.dataset.onboardingCommand = id;
      const key = document.createElement('kbd');
      key.textContent = buttons;
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
    ...controllerCommands(bindings).map(({ buttons, label }) => {
      const hint = document.createElement('span');
      const keyElement = document.createElement('kbd');
      keyElement.textContent = buttons;
      hint.append(keyElement, document.createTextNode(translate(appLocale, label)));
      return hint;
    }),
  );
}

function setLocale(locale: AppLocale) {
  appLocale = locale;
  languageSelect.value = locale;
  applyTranslations(document, locale);
  renderControllerStatus();
  renderControllerHints(currentBindings);
  renderOnboardingControls();
  renderPins();
  renderTvModeSettings();
  if (feedbackKey) feedback.textContent = translate(locale, feedbackKey, feedbackValues);
}

applyTranslations(document, appLocale);
renderControllerHints(currentBindings);
renderOnboardingControls();
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

function getInputHostname() {
  const url = normalizeUrl(urlInput.value);
  return url ? new URL(url).hostname.toLowerCase() : null;
}

function renderTvModeSettings() {
  const hostname = currentTvHostname;
  const hasSite = Boolean(hostname);
  tvModeSettings.hidden = !hasSite;
  tvModeEnabled.disabled = !hasSite;
  tvUrlField.hidden = !hasSite || !tvModeEnabled.checked;

  if (!hostname) {
    tvModeHint.textContent = translate(appLocale, 'tvEnterAddress');
  } else if (!tvModeEnabled.checked) {
    tvModeHint.textContent = getBuiltInTvUrl(hostname)
      ? translate(appLocale, 'tvYoutubeReady')
      : translate(appLocale, 'tvModeOff');
  } else if (tvUrlInput.value.trim()) {
    tvModeHint.textContent = translate(appLocale, 'tvEveryVisit');
  } else {
    tvModeHint.textContent = translate(appLocale, 'tvAndroidProfile');
  }
}

async function syncTvModeSettings(force = false) {
  const hostname = getInputHostname();
  if (!force && hostname === currentTvHostname) {
    renderTvModeSettings();
    return;
  }

  const sequence = ++tvProfileLoadSequence;
  currentTvHostname = hostname;
  currentTvProfile = null;
  tvModeEnabled.checked = false;
  tvModeEnabled.disabled = true;
  tvUrlInput.value = '';
  renderTvModeSettings();
  if (!hostname) return;

  try {
    const profile = await loadTvModeProfile(hostname);
    if (sequence !== tvProfileLoadSequence || getInputHostname() !== hostname) return;

    currentTvProfile = profile;
    const displayed = profile ?? createDefaultTvModeProfile(hostname);
    tvModeEnabled.checked = displayed.enabled;
    tvUrlInput.value = displayed.tvUrl;
    tvModeEnabled.disabled = false;
    renderTvModeSettings();
  } catch {
    if (sequence !== tvProfileLoadSequence) return;
    tvModeHint.textContent = translate(appLocale, 'tvCouldNotLoad');
  }
}

async function saveTvModeSettings(url: string) {
  const hostname = new URL(url).hostname.toLowerCase();
  if (hostname !== currentTvHostname || tvModeEnabled.disabled) await syncTvModeSettings(true);
  if (hostname !== currentTvHostname || tvModeEnabled.disabled) return false;

  const tvUrl = normalizeTvUrl(tvUrlInput.value, hostname);
  if (tvUrl === null) {
    setFeedback('tvSameHostname', {}, true);
    tvUrlInput.focus();
    return false;
  }

  const isCustomUrl = tvUrl !== getBuiltInTvUrl(hostname);
  if (!currentTvProfile && !tvModeEnabled.checked && !isCustomUrl) return true;

  try {
    const response = (await chrome.runtime.sendMessage({
      type: 'save-tv-mode-profile',
      profile: { hostname, enabled: tvModeEnabled.checked, tvUrl },
    })) as { ok?: boolean; error?: string } | undefined;

    if (!response?.ok) {
      setFeedback('tvCouldNotSave', {}, true);
      return false;
    }

    currentTvProfile = { hostname, enabled: tvModeEnabled.checked, tvUrl };
    return true;
  } catch {
    setFeedback('tvCouldNotSave', {}, true);
    return false;
  }
}

function displayHostname(url: string) {
  try {
    return new URL(url).hostname.replace(/^www\./, '');
  } catch {
    return url;
  }
}

function monogram(label: string) {
  return label
    .split(/[\s.·-]+/)
    .filter(Boolean)
    .slice(0, 2)
    .map((part) => part[0])
    .join('')
    .toUpperCase();
}

function createPin(site: PinnedSite) {
  const card = document.createElement('article');
  card.className = 'site-tile';

  const launch = document.createElement('button');
  launch.type = 'button';
  launch.className = 'tile-launch';
  launch.dataset.launch = site.id;
  launch.setAttribute(
    'aria-label',
    translate(appLocale, 'openShortcut', {
      label: site.label,
      host: displayHostname(site.url),
    }),
  );

  const mark = document.createElement('span');
  mark.className = 'tile-mark';
  mark.setAttribute('aria-hidden', 'true');
  mark.textContent = monogram(site.label);

  const faviconUrl = new URL(chrome.runtime.getURL('/_favicon/'));
  faviconUrl.searchParams.set('pageUrl', site.url);
  faviconUrl.searchParams.set('size', '32');
  const favicon = document.createElement('img');
  favicon.className = 'tile-favicon';
  favicon.alt = '';
  favicon.addEventListener('load', () => mark.replaceChildren(favicon), { once: true });
  favicon.src = faviconUrl.toString();

  const copy = document.createElement('span');
  copy.className = 'tile-copy';
  const name = document.createElement('span');
  name.className = 'tile-name';
  name.textContent = site.label;
  const address = document.createElement('span');
  address.className = 'tile-url';
  address.textContent = displayHostname(site.url);
  copy.append(name, address);
  launch.append(mark, copy);
  launch.addEventListener('click', () => window.location.assign(site.url));
  card.append(launch);

  if (editingPins) {
    const tools = document.createElement('div');
    tools.className = 'pin-tools';

    const edit = document.createElement('button');
    edit.type = 'button';
    edit.className = 'pin-tool';
    edit.textContent = translate(appLocale, 'edit');
    edit.setAttribute('aria-label', translate(appLocale, 'editShortcut', { label: site.label }));
    edit.addEventListener('click', () => editPin(site));

    const remove = document.createElement('button');
    remove.type = 'button';
    remove.className = 'pin-tool';
    remove.textContent = translate(appLocale, 'remove');
    remove.setAttribute(
      'aria-label',
      translate(appLocale, 'removeShortcut', { label: site.label }),
    );
    remove.addEventListener('click', () => void removePin(site.id, site.label));

    tools.append(edit, remove);
    card.append(tools);
  }

  return card;
}

function renderPins(focusFirst = false) {
  pinGrid.replaceChildren(...pinnedSites.map(createPin));
  pinGrid.classList.toggle('is-editing', editingPins);
  editPinsButton.textContent = translate(appLocale, editingPins ? 'done' : 'edit');

  if (!pinnedSites.length) {
    const empty = document.createElement('p');
    empty.className = 'empty-pins';
    empty.textContent = translate(appLocale, 'emptyShortcuts');
    pinGrid.append(empty);
  }

  if (focusFirst && !onboardingVisible) {
    const firstPin = pinGrid.querySelector<HTMLButtonElement>('[data-launch]');
    if (firstPin) firstPin.focus({ preventScroll: true });
    else urlInput.focus({ preventScroll: true });
  }
}

function clearPinForm() {
  editingPinId = null;
  urlInput.value = '';
  labelInput.value = '';
  savePinButton.textContent = translate(appLocale, 'addShortcut');
  void syncTvModeSettings(true);
}

function editPin(site: PinnedSite) {
  editingPinId = site.id;
  urlInput.value = site.url;
  labelInput.value = site.label;
  savePinButton.textContent = translate(appLocale, 'saveChanges');
  setFeedback('feedbackEditingShortcut', { label: site.label });
  void syncTvModeSettings(true);
  document.querySelector('#site-form')?.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
  urlInput.focus({ preventScroll: true });
}

async function removePin(id: string, label: string) {
  const nextSites = pinnedSites.filter((site) => site.id !== id);
  try {
    await savePinnedSites(nextSites);
  } catch {
    setFeedback('feedbackCouldNotRemoveShortcut', {}, true);
    return;
  }
  pinnedSites = nextSites;
  if (editingPinId === id) clearPinForm();
  renderPins();
  setFeedback('feedbackShortcutRemoved', { label });
}

async function savePin() {
  const url = normalizeUrl(urlInput.value);
  if (!url) {
    setFeedback('feedbackInvalidWebsite', {}, true);
    urlInput.focus();
    return;
  }

  const duplicate = pinnedSites.find((site) => site.url === url && site.id !== editingPinId);
  if (duplicate) {
    setFeedback('feedbackDuplicateShortcut', { label: duplicate.label }, true);
    return;
  }

  if (!(await saveTvModeSettings(url))) return;

  const site: PinnedSite = {
    id: editingPinId ?? crypto.randomUUID(),
    label: labelInput.value.trim() || displayHostname(url),
    url,
  };

  const nextSites = editingPinId
    ? pinnedSites.map((current) => (current.id === site.id ? site : current))
    : [...pinnedSites, site];

  try {
    await savePinnedSites(nextSites);
    pinnedSites = nextSites;
    renderPins();
    clearPinForm();
    setFeedback('feedbackShortcutSaved', { label: site.label });
  } catch {
    setFeedback('feedbackCouldNotSaveShortcut', {}, true);
  }
}

async function goToAddress() {
  const url = normalizeUrl(urlInput.value);
  if (!url) {
    setFeedback('feedbackInvalidWebsite', {}, true);
    urlInput.focus();
    return;
  }
  if (!(await saveTvModeSettings(url))) return;
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
    const firstKey = keyboard.querySelector<HTMLButtonElement>('[data-key]');
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
  else if (action.type === 'scroll') window.scrollBy(0, action.amount);
  else if (action.type === 'back') {
    if (activeSelectPicker?.open) activeSelectPicker.close('cancel');
    else void currentTabId.then((id) => sendTabAction('back', id));
  } else if (action.type === 'refresh') window.location.reload();
  else if (action.type === 'previous-tab')
    void currentTabId.then((id) => sendTabAction('prev-tab', id));
  else if (action.type === 'next-tab')
    void currentTabId.then((id) => sendTabAction('next-tab', id));
  else if (action.type === 'new-tab') void currentTabId.then((id) => sendTabAction('new-tab', id));
}

function handleGamepadState(state: { connected: boolean; enabled: boolean }) {
  currentControllerState = state;
  controllerStatus.dataset.connected = String(state.connected);
  controllerStatus.dataset.enabled = String(state.enabled);
  renderControllerStatus();
}

editPinsButton.addEventListener('click', () => {
  editingPins = !editingPins;
  if (!editingPins) clearPinForm();
  renderPins();
});

form.addEventListener('submit', (event) => {
  event.preventDefault();
  void goToAddress();
});

urlInput.addEventListener('input', () => void syncTvModeSettings());
tvModeEnabled.addEventListener('change', renderTvModeSettings);
savePinButton.addEventListener('click', () => void savePin());

void loadPinnedSites()
  .then((sites) => {
    pinnedSites = sites;
    renderPins(true);
  })
  .catch(() => setFeedback('feedbackCouldNotLoadShortcuts', {}, true));

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
    stopLocaleWatch();
  },
  { once: true },
);
