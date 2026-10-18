import {
  createDefaultTvModeProfile,
  getBuiltInTvUrl,
  loadTvModeProfile,
  normalizeTvUrl,
  type TvModeProfile,
} from '../../../lib/tv-mode';
import { normalizeUrl } from '../../../lib/url';
import { translate, type AppLocale, type TranslationKey } from '../../../lib/i18n';

type FeedbackValues = Record<string, string | number>;
type SetFeedback = (key: TranslationKey, values?: FeedbackValues, isError?: boolean) => void;

interface TvModeSettingsOptions {
  urlInput: HTMLInputElement;
  settings: HTMLElement;
  enabled: HTMLInputElement;
  hint: HTMLParagraphElement;
  urlField: HTMLDivElement;
  tvUrlInput: HTMLInputElement;
  getLocale: () => AppLocale;
  setFeedback: SetFeedback;
}

export type TvModeSaveResult = 'saved' | 'unchanged' | 'failed';

export function createTvModeSettings({
  urlInput,
  settings,
  enabled,
  hint,
  urlField,
  tvUrlInput,
  getLocale,
  setFeedback,
}: TvModeSettingsOptions) {
  let currentHostname: string | null = null;
  let currentProfile: TvModeProfile | null = null;
  let loadSequence = 0;

  function getInputHostname() {
    const url = normalizeUrl(urlInput.value);
    return url ? new URL(url).hostname.toLowerCase() : null;
  }

  function render() {
    const hasSite = Boolean(currentHostname);
    settings.hidden = !hasSite;
    enabled.disabled = !hasSite;
    urlField.hidden = !hasSite || !enabled.checked;

    if (!currentHostname) {
      hint.textContent = translate(getLocale(), 'tvEnterAddress');
    } else if (!enabled.checked) {
      hint.textContent = getBuiltInTvUrl(currentHostname)
        ? translate(getLocale(), 'tvYoutubeReady')
        : translate(getLocale(), 'tvModeOff');
    } else if (tvUrlInput.value.trim()) {
      hint.textContent = translate(getLocale(), 'tvEveryVisit');
    } else {
      hint.textContent = translate(getLocale(), 'tvAndroidProfile');
    }
  }

  async function sync(force = false) {
    const hostname = getInputHostname();
    if (!force && hostname === currentHostname) {
      render();
      return;
    }

    const sequence = ++loadSequence;
    currentHostname = hostname;
    currentProfile = null;
    enabled.checked = false;
    enabled.disabled = true;
    tvUrlInput.value = '';
    render();
    if (!hostname) return;

    try {
      const profile = await loadTvModeProfile(hostname);
      if (sequence !== loadSequence || getInputHostname() !== hostname) return;

      currentProfile = profile;
      const displayed = profile ?? createDefaultTvModeProfile(hostname);
      enabled.checked = displayed.enabled;
      tvUrlInput.value = displayed.tvUrl;
      enabled.disabled = false;
      render();
    } catch {
      if (sequence !== loadSequence) return;
      hint.textContent = translate(getLocale(), 'tvCouldNotLoad');
    }
  }

  async function saveForUrl(url: string): Promise<TvModeSaveResult> {
    const hostname = new URL(url).hostname.toLowerCase();
    if (hostname !== currentHostname || enabled.disabled) await sync(true);
    if (hostname !== currentHostname || enabled.disabled) return 'failed';

    const tvUrl = normalizeTvUrl(tvUrlInput.value, hostname);
    if (tvUrl === null) {
      setFeedback('tvSameHostname', {}, true);
      tvUrlInput.focus();
      return 'failed';
    }

    const isCustomUrl = tvUrl !== getBuiltInTvUrl(hostname);
    if (!currentProfile && !enabled.checked && !isCustomUrl) return 'unchanged';

    try {
      const response = (await chrome.runtime.sendMessage({
        type: 'save-tv-mode-profile',
        profile: { hostname, enabled: enabled.checked, tvUrl },
      })) as { ok?: boolean; error?: string } | undefined;

      if (!response?.ok) {
        setFeedback('tvCouldNotSave', {}, true);
        return 'failed';
      }

      currentProfile = { hostname, enabled: enabled.checked, tvUrl };
      return 'saved';
    } catch {
      setFeedback('tvCouldNotSave', {}, true);
      return 'failed';
    }
  }

  return { render, sync, saveForUrl };
}
