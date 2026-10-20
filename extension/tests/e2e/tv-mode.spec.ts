import { createServer, type Server } from 'node:http';
import { expect, test } from './fixtures';

async function hideFloatingKeyboard(page: import('@playwright/test').Page) {
  const hideButton = page.locator('[data-gamepad-desktop="keyboard"] [data-key-action="hide"]');
  if (await hideButton.isVisible()) await hideButton.click();
}

let siteServer: Server;
let siteOrigin = '';
let observedUserAgents: string[] = [];

async function openNewTab(context: import('@playwright/test').BrowserContext) {
  const page = await context.newPage();
  await page.goto('chrome://newtab/');
  await expect(page).toHaveURL(/chrome-extension:\/\/[^/]+\/newtab\.html/);
  await expect(page.locator('html')).toHaveAttribute('data-onboarding-ready', 'true');
  if (await page.locator('#onboarding-panel').isVisible()) {
    await page.locator('#onboarding-start').click();
    await expect(page.locator('#onboarding-panel')).toBeHidden();
  }
  return page;
}

async function configuredRedirects(context: import('@playwright/test').BrowserContext) {
  const worker = context.serviceWorkers()[0];
  if (!worker) throw new Error('Extension service worker is not running');
  return worker.evaluate(async () =>
    (await chrome.declarativeNetRequest.getDynamicRules()).flatMap(
      (rule) => rule.action.redirect?.url ?? [],
    ),
  );
}

async function savedTvProfiles(context: import('@playwright/test').BrowserContext) {
  const worker = context.serviceWorkers()[0];
  if (!worker) throw new Error('Extension service worker is not running');
  return worker.evaluate(
    async () => (await chrome.storage.local.get('tvModeProfiles')).tvModeProfiles,
  );
}

test.beforeEach(async () => {
  observedUserAgents = [];
  siteServer = createServer((request, response) => {
    observedUserAgents.push(request.headers['user-agent'] ?? '');
    const url = new URL(request.url ?? '/', `http://${request.headers.host}`);
    const body =
      url.pathname === '/ua'
        ? '<!doctype html><body><output id="ua"></output><script>document.querySelector("#ua").textContent = navigator.userAgent</script></body>'
        : '<!doctype html><button>TV landing page</button>';
    response.writeHead(200, { 'content-type': 'text/html' });
    response.end(body);
  });
  await new Promise<void>((resolve, reject) => {
    siteServer.once('error', reject);
    siteServer.listen(0, '127.0.0.1', resolve);
  });
  const address = siteServer.address();
  if (!address || typeof address === 'string') throw new Error('Could not start the TV test site');
  siteOrigin = `http://127.0.0.1:${address.port}`;
});

test.afterEach(async () => {
  siteServer.closeAllConnections();
  await new Promise<void>((resolve, reject) => {
    siteServer.close((error) => (error ? reject(error) : resolve()));
  });
});

test('redirects every visit to a same-host TV landing URL and persists the profile', async ({
  context,
}) => {
  const page = await openNewTab(context);
  await page.locator('#site-url').fill(`${siteOrigin}/watch?video=one`);
  await expect(page.locator('#tv-mode-enabled')).toBeEnabled();
  await page.locator('#tv-mode-enabled').check();
  await page.locator('#tv-url').fill(`${siteOrigin}/tv`);
  await hideFloatingKeyboard(page);
  await page.locator('.open-button').click();
  await expect(page).toHaveURL(`${siteOrigin}/tv`);

  await page.goto(`${siteOrigin}/watch?video=two`);
  await expect(page).toHaveURL(`${siteOrigin}/tv`);

  const settingsTab = await openNewTab(context);
  await settingsTab.locator('#site-url').fill(`${siteOrigin}/another-page`);
  await expect(settingsTab.locator('#tv-mode-enabled')).toBeEnabled();
  await expect(settingsTab.locator('#tv-mode-enabled')).toBeChecked();
  await expect(settingsTab.locator('#tv-url')).toHaveValue(`${siteOrigin}/tv`);
});

test('uses the Android TV WebView profile when no TV URL is configured', async ({ context }) => {
  const page = await openNewTab(context);
  await page.locator('#site-url').fill(`${siteOrigin}/ua`);
  await expect(page.locator('#tv-mode-enabled')).toBeEnabled();
  await page.locator('#tv-mode-enabled').check();
  await expect(page.locator('#tv-url')).toHaveValue('');
  await hideFloatingKeyboard(page);
  await page.locator('.open-button').click();
  await expect(page).toHaveURL(`${siteOrigin}/ua`);
  await expect(page.locator('#ua')).toContainText('Android 13; Android TV; wv');
  await expect.poll(() => observedUserAgents.at(-1)).toContain('Android 13; Android TV; wv');
});

test('rejects cross-host TV URLs without navigating', async ({ context }) => {
  const page = await openNewTab(context);
  await page.locator('#site-url').fill(`${siteOrigin}/watch`);
  await expect(page.locator('#tv-mode-enabled')).toBeEnabled();
  await page.locator('#tv-mode-enabled').check();
  await page.locator('#tv-url').fill('https://example.com/tv');
  await hideFloatingKeyboard(page);
  await page.locator('.open-button').click();

  await expect(page.locator('#feedback')).toContainText('same hostname');
  await expect(page).toHaveURL(/chrome-extension:\/\/[^/]+\/newtab\.html/);
});

test('uses YouTube’s built-in route and leaves Crunchyroll custom-only', async ({ context }) => {
  const youtubeTab = await openNewTab(context);
  await youtubeTab.locator('#site-url').fill('https://www.youtube.com/watch?v=one');
  await expect(youtubeTab.locator('#tv-mode-enabled')).toBeEnabled();
  await expect(youtubeTab.locator('#tv-mode-enabled')).not.toBeChecked();
  await youtubeTab.locator('#tv-mode-enabled').check();
  await expect(youtubeTab.locator('#tv-url')).toHaveValue('https://www.youtube.com/tv');
  await youtubeTab.locator('#save-pin').click();
  await expect(youtubeTab.locator('#feedback')).toContainText('saved to shortcuts');
  await expect(await configuredRedirects(context)).toContain('https://www.youtube.com/tv');

  const crunchyrollTab = await openNewTab(context);
  await crunchyrollTab.locator('#site-url').fill('https://www.crunchyroll.com/');
  await expect(crunchyrollTab.locator('#tv-mode-enabled')).toBeEnabled();
  await crunchyrollTab.locator('#tv-mode-enabled').check();
  await expect(crunchyrollTab.locator('#tv-url')).toHaveValue('');
  await crunchyrollTab.locator('#tv-url').fill('https://www.crunchyroll.com/tv');
  await crunchyrollTab.locator('#save-pin').click();
  await expect(crunchyrollTab.locator('#feedback')).toContainText('saved to shortcuts');
  await expect(await savedTvProfiles(context)).toContainEqual({
    hostname: 'www.crunchyroll.com',
    enabled: true,
    tvUrl: 'https://www.crunchyroll.com/tv',
  });
  await expect(await configuredRedirects(context)).toContain('https://www.crunchyroll.com/tv');
});
