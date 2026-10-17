import type { Page, BrowserContext } from '@playwright/test';
import { E2E_GAMEPAD_MESSAGE, type GamepadFrame } from '../../lib/gamepad';
import { expect, test } from './fixtures';

function makeFrame(pressed: number[] = [], axes: number[] = [0, 0, 0, 0]): GamepadFrame {
  return {
    id: 'E2E standard gamepad',
    connected: true,
    axes,
    buttons: Array.from({ length: 16 }, (_, index) => ({ pressed: pressed.includes(index) })),
  };
}

async function sendFrame(page: Page, frame: GamepadFrame, waitForSample = false) {
  await page.evaluate(
    ({ type, frame: nextFrame }) => window.postMessage({ type, frame: nextFrame }, '*'),
    { type: E2E_GAMEPAD_MESSAGE, frame },
  );
  if (waitForSample) {
    await page.evaluate(
      () =>
        new Promise<void>((resolve) =>
          requestAnimationFrame(() => requestAnimationFrame(() => resolve())),
        ),
    );
  }
}

async function openNewTab(context: BrowserContext) {
  const page = await context.newPage();
  await page.goto('chrome://newtab/');
  await expect(page).toHaveURL(/chrome-extension:\/\/[^/]+\/newtab\.html/);
  await expect(page.getByRole('heading', { name: /your next stop/i })).toBeVisible();
  return page;
}

test.beforeEach(async ({ context }) => {
  await context.route(/^https?:\/\/(alpha|beta)\.test\//, (route) =>
    route.fulfill({
      status: 200,
      contentType: 'text/html',
      body: '<!doctype html><button>Game control</button>',
    }),
  );
  await context.route('https://www.youtube.com/**', (route) =>
    route.fulfill({
      status: 200,
      contentType: 'text/html',
      body: '<!doctype html><button>Play video</button>',
    }),
  );
});

test('opens the navigator on first install', async ({ context }) => {
  await expect
    .poll(() =>
      context.pages().some((page) => /chrome-extension:\/\/[^/]+\/newtab\.html/.test(page.url())),
    )
    .toBe(true);
});

test('overrides every new tab and focuses the first starter shortcut', async ({ context }) => {
  const page = await openNewTab(context);
  const override = await page.evaluate(
    () => chrome.runtime.getManifest().chrome_url_overrides?.newtab,
  );

  expect(override).toBe('newtab.html');
  await expect(page.locator('[data-launch="ac-jr"]')).toBeFocused();
  await expect(page.locator('[data-launch="youtube"]')).toBeVisible();
  await expect(page.locator('[data-launch="google"]')).toBeVisible();
  await expect(page.locator('[data-launch="steam"]')).toBeVisible();

  const secondTab = await openNewTab(context);
  await expect(secondTab.locator('[data-launch="ac-jr"]')).toBeFocused();
});

test('keeps website entry and shortcut creation at the top of the new tab', async ({ context }) => {
  const page = await openNewTab(context);

  await expect(page.getByRole('heading', { name: 'New destination' })).toBeVisible();
  await expect(page.getByRole('textbox', { name: 'Website address' })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Open A', exact: true })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Add shortcut' })).toBeVisible();

  const destinationTop = await page
    .locator('.destination-bar')
    .evaluate((node) => node.getBoundingClientRect().top);
  const shortcutsTop = await page
    .locator('#pin-grid')
    .evaluate((node) => node.getBoundingClientRect().top);
  expect(destinationTop).toBeLessThan(shortcutsTop);

  const pageHeight = await page.evaluate(() => ({
    content: document.documentElement.scrollHeight,
    viewport: window.innerHeight,
  }));
  expect(pageHeight.content).toBeLessThanOrEqual(pageHeight.viewport);
});

test('gamepad can reach the address bar from the focused shortcut', async ({ context }) => {
  const page = await openNewTab(context);

  await sendFrame(page, makeFrame(), true);
  await sendFrame(page, makeFrame([12]));
  await expect(page.getByRole('button', { name: 'Edit', exact: true })).toBeFocused();

  await sendFrame(page, makeFrame(), true);
  await page.waitForTimeout(220);
  await sendFrame(page, makeFrame([12]));
  await expect(page.locator('#site-url')).toBeFocused();
});

test('gamepad moves across shortcuts and types on the virtual keyboard', async ({ context }) => {
  const page = await openNewTab(context);
  const status = page.locator('#controller-status');

  await sendFrame(page, makeFrame(), true);
  await expect(status).toContainText('Controller ready');
  await sendFrame(page, makeFrame([15]));
  await expect(page.locator('[data-launch="youtube"]')).toBeFocused();
  await sendFrame(page, makeFrame(), true);

  await page.locator('#site-url').focus();
  await page.locator('[data-key="q"]').focus();
  await sendFrame(page, makeFrame([0]));
  await expect(page.locator('#site-url')).toHaveValue('q');
  await sendFrame(page, makeFrame(), true);

  await page.locator('[data-key="."]').focus();
  await sendFrame(page, makeFrame([0]));
  await expect(page.locator('#site-url')).toHaveValue('q.');
  await sendFrame(page, makeFrame(), true);

  await page.locator('[data-key-action="backspace"]').focus();
  await sendFrame(page, makeFrame([0]));
  await expect(page.locator('#site-url')).toHaveValue('q');
  await sendFrame(page, makeFrame(), true);

  await page.locator('[data-key-action="shift"]').focus();
  await sendFrame(page, makeFrame([0]));
  await expect(page.locator('[data-key-action="shift"]')).toHaveAttribute('aria-pressed', 'true');
  await sendFrame(page, makeFrame(), true);
  await page.locator('[data-key="z"]').focus();
  await sendFrame(page, makeFrame([0]));
  await expect(page.locator('#site-url')).toHaveValue('qZ');
  await sendFrame(page, makeFrame(), true);

  await page.locator('[data-key-action="clear"]').focus();
  await sendFrame(page, makeFrame([0]));
  await expect(page.locator('#site-url')).toHaveValue('');
});

test('A opens the focused shortcut in the same tab', async ({ context }) => {
  const page = await openNewTab(context);
  const pageCount = context.pages().length;

  await sendFrame(page, makeFrame(), true);
  await sendFrame(page, makeFrame([15]));
  await expect(page.locator('[data-launch="youtube"]')).toBeFocused();
  await sendFrame(page, makeFrame(), true);
  await sendFrame(page, makeFrame([0]));

  await expect(page).toHaveURL('https://www.youtube.com/');
  expect(context.pages()).toHaveLength(pageCount);
});

test('shoulder buttons switch tabs from the navigator', async ({ context }) => {
  const navigatorPage = await openNewTab(context);
  const otherPage = await context.newPage();
  await otherPage.goto('https://alpha.test/second');
  await navigatorPage.bringToFront();

  await sendFrame(navigatorPage, makeFrame(), true);
  await sendFrame(navigatorPage, makeFrame([5]));
  await expect.poll(() => otherPage.evaluate(() => document.visibilityState)).toBe('visible');
});

test('opens URLs in the current tab and rejects non-web protocols', async ({ context }) => {
  const page = await openNewTab(context);
  const pageCount = context.pages().length;

  await page.locator('#site-url').fill('alpha.test/arcade');
  await page.locator('.open-button').focus();
  await sendFrame(page, makeFrame(), true);
  await sendFrame(page, makeFrame([0]));
  await expect(page).toHaveURL('https://alpha.test/arcade');
  expect(context.pages()).toHaveLength(pageCount);

  await page.goBack();
  await expect(page).toHaveURL(/chrome-extension:\/\/[^/]+\/newtab\.html/);
  await page.locator('#site-url').fill('javascript:alert(1)');
  await page.locator('.open-button').click();
  await expect(page.locator('#feedback')).toContainText('valid HTTP or HTTPS');
  await expect(page).toHaveURL(/chrome-extension:\/\/[^/]+\/newtab\.html/);
});

test('adds, edits, and removes pins while preserving changes after reload', async ({ context }) => {
  const page = await openNewTab(context);

  await page.locator('#site-url').fill('https://alpha.test/arcade');
  await page.locator('#site-label').fill('Alpha Arcade');
  await page.getByRole('button', { name: 'Add shortcut' }).click();
  await expect(page.getByRole('button', { name: 'Open Alpha Arcade, alpha.test' })).toBeVisible();

  await page.reload();
  await expect(page.getByRole('button', { name: /open alpha arcade/i })).toBeVisible();

  await page.getByRole('button', { name: 'Edit', exact: true }).click();
  await page.getByRole('button', { name: 'Edit Alpha Arcade' }).click();
  await page.locator('#site-label').fill('Alpha Games');
  await page.getByRole('button', { name: 'Save changes' }).click();
  await expect(page.getByRole('button', { name: /open alpha games/i })).toBeVisible();

  await page.reload();
  await expect(page.getByRole('button', { name: /open alpha games/i })).toBeVisible();
  await page.getByRole('button', { name: 'Edit', exact: true }).click();
  await page.getByRole('button', { name: 'Remove Alpha Games' }).click();
  await expect(page.getByRole('button', { name: /open alpha games/i })).toHaveCount(0);
  await page.reload();
  await expect(page.getByRole('button', { name: /open alpha games/i })).toHaveCount(0);
});
