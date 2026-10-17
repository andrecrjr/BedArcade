import { startGamepadController, type GamepadAction } from '../../lib/gamepad';
import { loadPinnedSites, savePinnedSites, type PinnedSite } from '../../lib/pins';
import { nearest } from '../../lib/spatial';
import { sendTabAction } from '../../lib/tab-actions';
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
const keyboard = document.querySelector<HTMLDivElement>('.key-rows');
const controllerStatus = element<HTMLDivElement>('controller-status');

let pinnedSites: PinnedSite[] = [];
let editingPins = false;
let editingPinId: string | null = null;
let shiftEnabled = false;
let typingInput = urlInput;
const currentTabId = chrome.tabs
  .getCurrent()
  .then((tab) => tab?.id)
  .catch(() => undefined);

function setFeedback(message: string, isError = false) {
  feedback.textContent = message;
  feedback.dataset.error = String(isError);
}

function hostname(url: string) {
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
  launch.setAttribute('aria-label', `Open ${site.label}, ${hostname(site.url)}`);

  const mark = document.createElement('span');
  mark.className = 'tile-mark';
  mark.setAttribute('aria-hidden', 'true');
  mark.textContent = monogram(site.label);

  const copy = document.createElement('span');
  copy.className = 'tile-copy';
  const name = document.createElement('span');
  name.className = 'tile-name';
  name.textContent = site.label;
  const address = document.createElement('span');
  address.className = 'tile-url';
  address.textContent = hostname(site.url);
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
}

function editPin(site: PinnedSite) {
  editingPinId = site.id;
  urlInput.value = site.url;
  labelInput.value = site.label;
  savePinButton.textContent = 'Save changes';
  setFeedback(`Editing ${site.label}`);
  document
    .querySelector('.destination-bar')
    ?.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
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

  const site: PinnedSite = {
    id: editingPinId ?? crypto.randomUUID(),
    label: labelInput.value.trim() || hostname(url),
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

function goToAddress() {
  const url = normalizeUrl(urlInput.value);
  if (!url) {
    setFeedback('Enter a valid HTTP or HTTPS website address.', true);
    urlInput.focus();
    return;
  }
  window.location.assign(url);
}

function insertText(input: HTMLInputElement, text: string) {
  const start = input.selectionStart ?? input.value.length;
  const end = input.selectionEnd ?? input.value.length;
  input.setRangeText(text, start, end, 'end');
  input.dispatchEvent(new Event('input', { bubbles: true }));
}

function backspace(input: HTMLInputElement) {
  let start = input.selectionStart ?? input.value.length;
  const end = input.selectionEnd ?? input.value.length;
  if (start === end && start > 0) start -= 1;
  input.setRangeText('', start, end, 'end');
  input.dispatchEvent(new Event('input', { bubbles: true }));
}

function activateFocusedElement() {
  const active = document.activeElement;
  if (active instanceof HTMLElement && active !== document.body) active.click();
}

function moveFocus(direction: 'up' | 'down' | 'left' | 'right') {
  const active = document.activeElement instanceof HTMLElement ? document.activeElement : null;
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
  else if (action.type === 'previous-tab')
    void currentTabId.then((id) => sendTabAction('prev-tab', id));
  else if (action.type === 'next-tab')
    void currentTabId.then((id) => sendTabAction('next-tab', id));
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
  goToAddress();
});

element<HTMLButtonElement>('save-pin').addEventListener('click', () => void savePin());

urlInput.addEventListener('focus', () => (typingInput = urlInput));
labelInput.addEventListener('focus', () => (typingInput = labelInput));

keyboard?.addEventListener('click', (event) => {
  const button = (event.target as HTMLElement).closest<HTMLButtonElement>('button');
  if (!button) return;

  const action = button.dataset.keyAction;
  if (action === 'shift') {
    shiftEnabled = !shiftEnabled;
    button.setAttribute('aria-pressed', String(shiftEnabled));
    keyboard.querySelectorAll<HTMLButtonElement>('[data-key]').forEach((key) => {
      const value = key.dataset.key ?? '';
      if (/^[a-z]$/i.test(value))
        key.textContent = shiftEnabled ? value.toUpperCase() : value.toLowerCase();
    });
    return;
  }
  if (action === 'clear') {
    typingInput.value = '';
    typingInput.dispatchEvent(new Event('input', { bubbles: true }));
    setFeedback('Address cleared.');
    return;
  }
  if (action === 'backspace') {
    backspace(typingInput);
    return;
  }

  const key = button.dataset.key;
  if (key !== undefined) {
    insertText(typingInput, /^[a-z]$/i.test(key) && shiftEnabled ? key.toUpperCase() : key);
    if (shiftEnabled) {
      shiftEnabled = false;
      const shift = keyboard.querySelector<HTMLButtonElement>('[data-key-action="shift"]');
      shift?.setAttribute('aria-pressed', 'false');
      keyboard.querySelectorAll<HTMLButtonElement>('[data-key]').forEach((letter) => {
        const value = letter.dataset.key ?? '';
        if (/^[a-z]$/i.test(value)) letter.textContent = value.toLowerCase();
      });
    }
  }
});

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

window.addEventListener('pagehide', stopGamepad, { once: true });
