import { translate, type AppLocale, type TranslationKey } from '../../../lib/i18n';
import { loadPinnedSites, savePinnedSites, type PinnedSite } from '../../../lib/pins';
import { normalizeUrl } from '../../../lib/url';

type FeedbackValues = Record<string, string | number>;
type SetFeedback = (key: TranslationKey, values?: FeedbackValues, isError?: boolean) => void;

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

interface PinsFeatureOptions {
  pinGrid: HTMLDivElement;
  form: HTMLFormElement;
  urlInput: HTMLInputElement;
  labelInput: HTMLInputElement;
  savePinButton: HTMLButtonElement;
  editPinsButton: HTMLButtonElement;
  getLocale: () => AppLocale;
  canFocusFirstPin: () => boolean;
  setFeedback: SetFeedback;
  onUrlChange: () => void;
}

export function createPinsFeature({
  pinGrid,
  form,
  urlInput,
  labelInput,
  savePinButton,
  editPinsButton,
  getLocale,
  canFocusFirstPin,
  setFeedback,
  onUrlChange,
}: PinsFeatureOptions) {
  let pinnedSites: PinnedSite[] = [];
  let editingPins = false;
  let editingPinId: string | null = null;

  function clearForm() {
    editingPinId = null;
    urlInput.value = '';
    labelInput.value = '';
    savePinButton.textContent = translate(getLocale(), 'addShortcut');
    onUrlChange();
  }

  function editPin(site: PinnedSite) {
    editingPinId = site.id;
    urlInput.value = site.url;
    labelInput.value = site.label;
    savePinButton.textContent = translate(getLocale(), 'saveChanges');
    setFeedback('feedbackEditingShortcut', { label: site.label });
    onUrlChange();
    form.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
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
    if (editingPinId === id) clearForm();
    render();
    setFeedback('feedbackShortcutRemoved', { label });
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
      translate(getLocale(), 'openShortcut', {
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
      edit.textContent = translate(getLocale(), 'edit');
      edit.setAttribute(
        'aria-label',
        translate(getLocale(), 'editShortcut', { label: site.label }),
      );
      edit.addEventListener('click', () => editPin(site));

      const remove = document.createElement('button');
      remove.type = 'button';
      remove.className = 'pin-tool';
      remove.textContent = translate(getLocale(), 'remove');
      remove.setAttribute(
        'aria-label',
        translate(getLocale(), 'removeShortcut', { label: site.label }),
      );
      remove.addEventListener('click', () => void removePin(site.id, site.label));

      tools.append(edit, remove);
      card.append(tools);
    }

    return card;
  }

  function render(focusFirst = false) {
    pinGrid.replaceChildren(...pinnedSites.map(createPin));
    pinGrid.classList.toggle('is-editing', editingPins);
    editPinsButton.textContent = translate(getLocale(), editingPins ? 'done' : 'edit');

    if (!pinnedSites.length) {
      const empty = document.createElement('p');
      empty.className = 'empty-pins';
      empty.textContent = translate(getLocale(), 'emptyShortcuts');
      pinGrid.append(empty);
    }

    if (focusFirst && canFocusFirstPin()) {
      const firstPin = pinGrid.querySelector<HTMLButtonElement>('[data-launch]');
      if (firstPin) firstPin.focus({ preventScroll: true });
      else urlInput.focus({ preventScroll: true });
    }
  }

  async function load() {
    try {
      pinnedSites = await loadPinnedSites();
      render(true);
    } catch {
      setFeedback('feedbackCouldNotLoadShortcuts', {}, true);
    }
  }

  function readDraft(): PinnedSite | null {
    const url = normalizeUrl(urlInput.value);
    if (!url) {
      setFeedback('feedbackInvalidWebsite', {}, true);
      urlInput.focus();
      return null;
    }

    const duplicate = pinnedSites.find((site) => site.url === url && site.id !== editingPinId);
    if (duplicate) {
      setFeedback('feedbackDuplicateShortcut', { label: duplicate.label }, true);
      return null;
    }

    return {
      id: editingPinId ?? crypto.randomUUID(),
      label: labelInput.value.trim() || displayHostname(url),
      url,
    };
  }

  async function save(site: PinnedSite) {
    const nextSites = editingPinId
      ? pinnedSites.map((current) => (current.id === site.id ? site : current))
      : [...pinnedSites, site];

    try {
      await savePinnedSites(nextSites);
    } catch {
      setFeedback('feedbackCouldNotSaveShortcut', {}, true);
      return false;
    }

    pinnedSites = nextSites;
    render();
    clearForm();
    setFeedback('feedbackShortcutSaved', { label: site.label });
    return true;
  }

  function toggleEditing() {
    editingPins = !editingPins;
    if (!editingPins) clearForm();
    render();
  }

  return { load, render, readDraft, save, toggleEditing };
}
