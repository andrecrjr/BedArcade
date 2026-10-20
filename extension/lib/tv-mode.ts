import { normalizeUrl } from './url';

export interface TvModeProfile {
  hostname: string;
  enabled: boolean;
  tvUrl: string;
}

export interface SaveTvModeProfileMessage {
  type: 'save-tv-mode-profile';
  profile: unknown;
}

export const TV_MODE_STORAGE_KEY = 'tvModeProfiles';
export const TV_MODE_RULE_ID_START = 30_000;
export const TV_MODE_RULE_ID_END = 39_999;
export const TV_MODE_SCRIPT_ID = 'tv-mode-user-agent';
export const ANDROID_TV_WEBVIEW_USER_AGENT =
  'Mozilla/5.0 (Linux; Android 13; Android TV; wv) AppleWebKit/537.36 (KHTML, like Gecko) Version/4.0 Chrome/120.0.0.0 Safari/537.36';

const BUILT_IN_TV_URLS: Record<string, string> = {
  'youtube.com': 'https://youtube.com/tv',
  'www.youtube.com': 'https://www.youtube.com/tv',
};

export function getBuiltInTvUrl(hostname: string) {
  return BUILT_IN_TV_URLS[hostname.toLowerCase()] ?? '';
}

export function createDefaultTvModeProfile(hostname: string): TvModeProfile {
  return {
    hostname: hostname.toLowerCase(),
    enabled: false,
    tvUrl: getBuiltInTvUrl(hostname),
  };
}

export function normalizeTvUrl(input: string, hostname: string): string | null {
  if (!input.trim()) return '';

  const value = normalizeUrl(input);
  if (!value) return null;

  try {
    const url = new URL(value);
    if (url.hostname.toLowerCase() !== hostname.toLowerCase()) return null;
    if (url.username || url.password) return null;
    url.hash = '';
    return url.href;
  } catch {
    return null;
  }
}

export function normalizeTvModeProfile(value: unknown): TvModeProfile | null {
  if (typeof value !== 'object' || value === null) return null;
  const profile = value as Record<string, unknown>;
  if (
    typeof profile.hostname !== 'string' ||
    !profile.hostname ||
    typeof profile.enabled !== 'boolean' ||
    typeof profile.tvUrl !== 'string'
  ) {
    return null;
  }

  try {
    const host = new URL(`https://${profile.hostname}/`);
    if (
      host.hostname.toLowerCase() !== profile.hostname.toLowerCase() ||
      host.port ||
      host.username ||
      host.password
    ) {
      return null;
    }

    const tvUrl = normalizeTvUrl(profile.tvUrl, host.hostname);
    if (tvUrl === null) return null;

    return {
      hostname: host.hostname.toLowerCase(),
      enabled: profile.enabled,
      tvUrl,
    };
  } catch {
    return null;
  }
}

export function isSaveTvModeProfileMessage(value: unknown): value is SaveTvModeProfileMessage {
  return (
    typeof value === 'object' &&
    value !== null &&
    (value as Record<string, unknown>).type === 'save-tv-mode-profile'
  );
}

export async function loadTvModeProfiles(): Promise<TvModeProfile[]> {
  const result = await chrome.storage.local.get(TV_MODE_STORAGE_KEY);
  const stored = result[TV_MODE_STORAGE_KEY] as unknown;
  if (!Array.isArray(stored)) return [];
  return stored.map(normalizeTvModeProfile).filter((profile) => profile !== null);
}

export async function loadTvModeProfile(hostname: string): Promise<TvModeProfile | null> {
  const profiles = await loadTvModeProfiles();
  return profiles.find((profile) => profile.hostname === hostname.toLowerCase()) ?? null;
}

export async function persistTvModeProfile(profile: TvModeProfile) {
  const profiles = await loadTvModeProfiles();
  const next = profiles.filter((current) => current.hostname !== profile.hostname);
  next.push(profile);
  await chrome.storage.local.set({ [TV_MODE_STORAGE_KEY]: next });
}

export function buildTvModeRules(profiles: TvModeProfile[]): chrome.declarativeNetRequest.Rule[] {
  const enabledProfiles = profiles
    .filter((profile) => profile.enabled)
    .toSorted((a, b) => a.hostname.localeCompare(b.hostname));

  if (enabledProfiles.length * 3 > TV_MODE_RULE_ID_END - TV_MODE_RULE_ID_START + 1) {
    throw new Error('Too many TV mode sites are enabled.');
  }

  const rules: chrome.declarativeNetRequest.Rule[] = [];
  enabledProfiles.forEach((profile, index) => {
    const ruleId = TV_MODE_RULE_ID_START + index * 3;
    const hostFilters = ['http', 'https'].map((scheme) => `|${scheme}://${profile.hostname}^*`);

    if (!profile.tvUrl) {
      rules.push(
        ...hostFilters.map((urlFilter, filterIndex) => ({
          id: ruleId + filterIndex + 1,
          priority: 1,
          action: {
            type: 'modifyHeaders' as const,
            requestHeaders: [
              {
                header: 'user-agent',
                operation: 'set' as const,
                value: ANDROID_TV_WEBVIEW_USER_AGENT,
              },
            ],
          },
          condition: { urlFilter },
        })),
      );
      return;
    }

    const landingUrl = new URL(profile.tvUrl);
    landingUrl.hash = '';

    rules.push(
      {
        id: ruleId,
        priority: 2,
        action: { type: 'allow' },
        condition: {
          urlFilter: `|${landingUrl.href}|`,
          resourceTypes: ['main_frame'],
        },
      },
      ...hostFilters.map((urlFilter, filterIndex) => ({
        id: ruleId + filterIndex + 1,
        priority: 1,
        action: { type: 'redirect' as const, redirect: { url: landingUrl.href } },
        condition: { urlFilter, resourceTypes: ['main_frame' as const] },
      })),
    );
  });

  return rules;
}
