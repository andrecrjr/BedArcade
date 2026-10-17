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
import { sendTabAction } from '../../lib/tab-actions';
import {
  createDefaultTvModeProfile,
  getBuiltInTvUrl,
  loadTvModeProfile,
  normalizeTvUrl,
  type TvModeProfile,
} from '../../lib/tv-mode';
import { normalizeUrl } from '../../lib/url';
import './style.css';

function element<T extends HTMLElement>(id: string): T {
  const found = document.getElementById(id);
  if (!found) throw new Error(`Missing #${id}`);
  return found as T;
}

const pinGrid = element<HTMLDivElement>('pin-grid');
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
const stopFloatingKeyboard = attachFloatingKeyboard({ urlShortcuts: true });

let pinnedSites: PinnedSite[] = [];
let editingPins = false;
let editingPinId: string | null = null;
let currentTvHostname: string | null = null;
let currentTvProfile: TvModeProfile | null = null;
let tvProfileLoadSequence = 0;
const currentTabId = chrome.tabs
  .getCurrent()
  .then((tab) => tab?.id)
  .catch(() => undefined);

function setFeedback(message: string, isError = false) {
  feedback.textContent = message;
  feedback.dataset.error = String(isError);
}

function renderControllerHints(bindings: GamepadBindings) {
  const hints: [string, string][] = [
    [
      `${gamepadButtonName(bindings.moveUp)}/${gamepadButtonName(bindings.moveDown)}/${gamepadButtonName(bindings.moveLeft)}/${gamepadButtonName(bindings.moveRight)}`,
      'MOVE',
    ],
    [gamepadButtonName(bindings.activate), 'SELECT'],
    [gamepadButtonName(bindings.back), 'BACK'],
    [
      `${gamepadButtonName(bindings.previousTab)}/${gamepadButtonName(bindings.nextTab)}`,
      'SWITCH TABS',
    ],
    [
      `${gamepadButtonName(bindings.refreshModifier)}+${gamepadButtonName(bindings.refreshTrigger)}`,
      'REFRESH',
    ],
    [gamepadButtonName(bindings.newTab), 'NEW TAB'],
    [gamepadButtonName(bindings.pause), 'PAUSE'],
  ];

  controllerHints.replaceChildren(
    ...hints.map(([button, action]) => {
      const hint = document.createElement('span');
      const key = document.createElement('kbd');
      key.textContent = button;
      hint.append(key, document.createTextNode(action));
      return hint;
    }),
  );
}

renderControllerHints(DEFAULT_GAMEPAD_BINDINGS);
let controllerHintsChanged = false;
const onGamepadBindingsChanged = (
  changes: { [key: string]: chrome.storage.StorageChange },
  areaName: string,
) => {
  const change = changes[GAMEPAD_BINDINGS_STORAGE_KEY];
  if (areaName !== 'local' || !change) return;
  controllerHintsChanged = true;
  renderControllerHints(normalizeGamepadBindings(change.newValue));
};
try {
  chrome.storage.onChanged.addListener(onGamepadBindingsChanged);
} catch {}
void loadGamepadBindings()
  .then((bindings) => {
    if (!controllerHintsChanged) renderControllerHints(bindings);
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
    tvModeHint.textContent = 'Enter a website address to configure TV mode.';
  } else if (!tvModeEnabled.checked) {
    tvModeHint.textContent = getBuiltInTvUrl(hostname)
      ? 'YouTube’s built-in TV route is ready when enabled.'
      : 'TV mode is off for this site.';
  } else if (tvUrlInput.value.trim()) {
    tvModeHint.textContent = 'Every visit will open the TV landing URL.';
  } else {
    tvModeHint.textContent = 'Uses the Android TV WebView profile.';
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
    tvModeHint.textContent = 'Could not load this site’s TV settings.';
  }
}

async function saveTvModeSettings(url: string) {
  const hostname = new URL(url).hostname.toLowerCase();
  if (hostname !== currentTvHostname || tvModeEnabled.disabled) await syncTvModeSettings(true);
  if (hostname !== currentTvHostname || tvModeEnabled.disabled) return false;

  const tvUrl = normalizeTvUrl(tvUrlInput.value, hostname);
  if (tvUrl === null) {
    setFeedback('The TV landing URL must use the same hostname, or be left blank.', true);
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
      setFeedback(response?.error ?? 'Could not save TV mode settings.', true);
      return false;
    }

    currentTvProfile = { hostname, enabled: tvModeEnabled.checked, tvUrl };
    return true;
  } catch {
    setFeedback('Could not save TV mode settings.', true);
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
  launch.setAttribute('aria-label', `Open ${site.label}, ${displayHostname(site.url)}`);

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
    edit.textContent = 'Edit';
    edit.setAttribute('aria-label', `Edit ${site.label}`);
    edit.addEventListener('click', () => editPin(site));

    const remove = document.createElement('button');
    remove.type = 'button';
    remove.className = 'pin-tool';
    remove.textContent = 'Remove';
    remove.setAttribute('aria-label', `Remove ${site.label}`);
    remove.addEventListener('click', () => void removePin(site.id, site.label));

    tools.append(edit, remove);
    card.append(tools);
  }

  return card;
}

function renderPins(focusFirst = false) {
  pinGrid.replaceChildren(...pinnedSites.map(createPin));
  pinGrid.classList.toggle('is-editing', editingPins);
  editPinsButton.textContent = editingPins ? 'Done' : 'Edit';

  if (!pinnedSites.length) {
    const empty = document.createElement('p');
    empty.className = 'empty-pins';
    empty.textContent = 'No shortcuts yet. Add a website using the address bar above.';
    pinGrid.append(empty);
  }

  if (focusFirst) {
    const firstPin = pinGrid.querySelector<HTMLButtonElement>('[data-launch]');
    if (firstPin) firstPin.focus({ preventScroll: true });
    else urlInput.focus({ preventScroll: true });
  }
}

function clearPinForm() {
  editingPinId = null;
  urlInput.value = '';
  labelInput.value = '';
  savePinButton.textContent = 'Add shortcut';
  void syncTvModeSettings(true);
}

function editPin(site: PinnedSite) {
  editingPinId = site.id;
  urlInput.value = site.url;
  labelInput.value = site.label;
  savePinButton.textContent = 'Save changes';
  setFeedback(`Editing ${site.label}`);
  void syncTvModeSettings(true);
  document.querySelector('#site-form')?.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
  urlInput.focus({ preventScroll: true });
}

async function removePin(id: string, label: string) {
  const nextSites = pinnedSites.filter((site) => site.id !== id);
  try {
    await savePinnedSites(nextSites);
  } catch {
    setFeedback('Could not remove this shortcut. Please try again.', true);
    return;
  }
  pinnedSites = nextSites;
  if (editingPinId === id) clearPinForm();
  renderPins();
  setFeedback(`${label} removed from shortcuts.`);
}

async function savePin() {
  const url = normalizeUrl(urlInput.value);
  if (!url) {
    setFeedback('Enter a valid HTTP or HTTPS website address.', true);
    urlInput.focus();
    return;
  }

  const duplicate = pinnedSites.find((site) => site.url === url && site.id !== editingPinId);
  if (duplicate) {
    setFeedback(`${duplicate.label} is already pinned.`, true);
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
    setFeedback(`${site.label} saved to shortcuts.`);
  } catch {
    setFeedback('Could not save this shortcut. Please try again.', true);
  }
}

async function goToAddress() {
  const url = normalizeUrl(urlInput.value);
  if (!url) {
    setFeedback('Enter a valid HTTP or HTTPS website address.', true);
    urlInput.focus();
    return;
  }
  if (!(await saveTvModeSettings(url))) return;
  window.location.assign(url);
}

function activateFocusedElement() {
  const active = document.activeElement;
  if (active instanceof HTMLElement && active !== document.body) active.click();
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
  else if (action.type === 'back') void currentTabId.then((id) => sendTabAction('back', id));
  else if (action.type === 'refresh') window.location.reload();
  else if (action.type === 'previous-tab')
    void currentTabId.then((id) => sendTabAction('prev-tab', id));
  else if (action.type === 'next-tab')
    void currentTabId.then((id) => sendTabAction('next-tab', id));
  else if (action.type === 'new-tab') void currentTabId.then((id) => sendTabAction('new-tab', id));
}

function handleGamepadState(state: { connected: boolean; enabled: boolean }) {
  controllerStatus.dataset.connected = String(state.connected);
  controllerStatus.dataset.enabled = String(state.enabled);
  if (!state.connected) {
    controllerStatus.textContent = 'Press any controller button to connect';
  } else if (!state.enabled) {
    controllerStatus.textContent = 'Controller paused · Press Start to resume';
  } else {
    controllerStatus.textContent = 'Controller ready · D-pad move · A open · B back';
  }
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
  .catch(() => setFeedback('Could not load shortcuts from this browser profile.', true));

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
  },
  { once: true },
);
