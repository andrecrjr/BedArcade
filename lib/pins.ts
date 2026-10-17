export interface PinnedSite {
  id: string;
  label: string;
  url: string;
}

const PINNED_SITES_KEY = 'pinnedSites';

export const DEFAULT_PINS: readonly PinnedSite[] = [
  { id: 'ac-jr', label: 'AC-JR', url: 'https://ac-jr.com/' },
  { id: 'youtube', label: 'YouTube', url: 'https://www.youtube.com/' },
  { id: 'google', label: 'Google', url: 'https://www.google.com/' },
  { id: 'steam', label: 'Steam', url: 'https://store.steampowered.com/' },
  { id: 'crazygames', label: 'CrazyGames', url: 'https://www.crazygames.com/' },
  {
    id: 'itch-gamepad',
    label: 'itch.io · Gamepad games',
    url: 'https://itch.io/games/input-gamepad/tag-browser',
  },
];

function isPinnedSite(value: unknown): value is PinnedSite {
  if (typeof value !== 'object' || value === null) return false;
  const site = value as Record<string, unknown>;
  if (
    typeof site.id !== 'string' ||
    typeof site.label !== 'string' ||
    typeof site.url !== 'string'
  ) {
    return false;
  }

  try {
    const url = new URL(site.url);
    return url.protocol === 'http:' || url.protocol === 'https:';
  } catch {
    return false;
  }
}

export async function loadPinnedSites(): Promise<PinnedSite[]> {
  const result = await chrome.storage.local.get(PINNED_SITES_KEY);
  const stored = result[PINNED_SITES_KEY] as unknown;

  if (stored === undefined) {
    const defaults = DEFAULT_PINS.map((site) => ({ ...site }));
    await savePinnedSites(defaults);
    return defaults;
  }

  return Array.isArray(stored) ? stored.filter(isPinnedSite) : [];
}

export async function savePinnedSites(sites: PinnedSite[]) {
  await chrome.storage.local.set({ [PINNED_SITES_KEY]: sites });
}
