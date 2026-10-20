const SCHEME = /^([a-z][a-z\d+.-]*):/i;
const HOST_WITH_PORT = /^(?:\[[\da-f:.]+\]|[^/?#:\s]+):\d+(?:[/?#]|$)/i;

export function normalizeUrl(input: string): string | null {
  const value = input.trim();
  if (!value || /^[/?#]/.test(value)) return null;

  const scheme = SCHEME.exec(value)?.[1]?.toLowerCase();
  const hasHostPort = HOST_WITH_PORT.test(value);
  const candidate = scheme && !hasHostPort ? value : `https://${value}`;

  try {
    const url = new URL(candidate);
    if ((url.protocol !== 'http:' && url.protocol !== 'https:') || !url.hostname) return null;
    return url.href;
  } catch {
    return null;
  }
}
