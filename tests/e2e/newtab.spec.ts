import type { Page, BrowserContext } from '@playwright/test';
import { E2E_GAMEPAD_MESSAGE, type GamepadFrame } from '../../lib/gamepad';
import { DEFAULT_GAMEPAD_BINDINGS } from '../../lib/gamepad-bindings';
import { detectAppLocale } from '../../lib/i18n';
import { onboardingCompletionAfterInstall } from '../../lib/onboarding';
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
  await expect(page.locator('html')).toHaveAttribute('data-onboarding-ready', 'true');
  if (await page.locator('#onboarding-panel').isVisible()) {
    await page.locator('#onboarding-start').click();
    await expect(page.locator('#onboarding-panel')).toBeHidden();
  }
  await expect(page.getByRole('heading', { name: /choose your next world/i })).toBeVisible();
  return page;
}

test.beforeEach(async ({ context }) => {
  await context.route(/^https?:\/\/(alpha|beta)\.test\//, async (route) => {
    const url = new URL(route.request().url());
    if (url.pathname === '/favicon.svg') {
      await route.fulfill({
        status: 200,
        contentType: 'image/svg+xml',
        body: '<svg xmlns="http://www.w3.org/2000/svg" width="32" height="32"><rect width="32" height="32" fill="#8fffc7"/></svg>',
      });
      return;
    }

    await route.fulfill({
      status: 200,
      contentType: 'text/html',
      body:
        url.pathname === '/text-fields'
          ? '<!doctype html><input id="text-input" type="text"><textarea id="message"></textarea>'
          : '<!doctype html><link rel="icon" href="/favicon.svg"><button>Game control</button>',
    });
  });
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

  const page = context.pages().find((current) => /newtab\.html/.test(current.url()));
  if (!page) throw new Error('The first-install launcher did not open');
  await expect(page.locator('#onboarding-panel')).toBeVisible();
  await expect(page.locator('[data-onboarding-card]')).toHaveCount(3);
  await expect(page.locator('[data-onboarding-command]')).toHaveCount(9);
  await expect(page.locator('[data-command-label]')).toHaveText([
    'Move focus',
    'Scroll page',
    'Open / select',
    'Go back',
    'Previous tab',
    'Next tab',
    'Reload page',
    'Open new tab',
    'Pause / resume',
  ]);
  await expect(page.locator('#controller-hints span')).toHaveCount(9);
  await expect(page.locator('#language-select option')).toHaveCount(3);
  await expect
    .poll(() =>
      page.evaluate(
        async () => (await chrome.storage.local.get('onboardingCompleted')).onboardingCompleted,
      ),
    )
    .toBe(false);

  await page.locator('#language-select').selectOption('es');
  await expect(page.locator('[data-command-label]')).toHaveText([
    'Mover foco',
    'Desplazar página',
    'Abrir / seleccionar',
    'Volver',
    'Pestaña anterior',
    'Pestaña siguiente',
    'Recargar página',
    'Abrir pestaña nueva',
    'Pausar / reanudar',
  ]);

  await page.locator('#language-select').selectOption('pt-BR');
  await expect(page.locator('[data-command-label]')).toHaveText([
    'Mover foco',
    'Rolar a página',
    'Abrir / selecionar',
    'Voltar',
    'Aba anterior',
    'Próxima aba',
    'Recarregar página',
    'Abrir nova aba',
    'Pausar / retomar',
  ]);

  await page.evaluate((bindings) => chrome.storage.local.set({ gamepadBindings: bindings }), {
    ...DEFAULT_GAMEPAD_BINDINGS,
    activate: 3,
  });
  await expect(page.locator('[data-onboarding-command="activate"] kbd')).toHaveText('Y');
});

test('keeps existing profiles out of the first-install tutorial after updates', () => {
  expect(onboardingCompletionAfterInstall('install', undefined)).toBe(false);
  expect(onboardingCompletionAfterInstall('update', undefined)).toBe(true);
  expect(onboardingCompletionAfterInstall('update', false)).toBe(null);
  expect(onboardingCompletionAfterInstall('update', true)).toBe(null);
});

test('detects the first supported browser language', () => {
  expect(detectAppLocale(['pt-BR', 'en-US'])).toBe('pt-BR');
  expect(detectAppLocale(['es-ES', 'en-US'])).toBe('es');
  expect(detectAppLocale(['en-US', 'pt-BR'])).toBe('en');
  expect(detectAppLocale(['fr-FR'])).toBe('en');
});

test('switches every extension surface to the chosen language and remembers it', async ({
  context,
}) => {
  const page = await openNewTab(context);
  await page.locator('#language-select').selectOption('es');
  await expect(page.locator('html')).toHaveAttribute('lang', 'es');
  await expect(page.getByRole('heading', { name: /Elige tu próximo mundo/i })).toBeVisible();

  await page.locator('#settings-link').click();
  await expect(page).toHaveURL(/settings\.html/);
  await expect(page.locator('html')).toHaveAttribute('lang', 'es');
  await expect(page.getByRole('heading', { name: 'Atajos del mando' })).toBeVisible();
  await expect(page.locator('#language-select')).toHaveValue('es');

  await page.goto('https://alpha.test/text-fields');
  const status = page.locator('[data-gamepad-navigator="status"]');
  const keyboard = page.locator('[data-gamepad-navigator="keyboard"]');
  await sendFrame(page, makeFrame(), true);
  await expect(status).toContainText('Mando activo');
  await page.locator('#text-input').focus();
  await expect(keyboard).toBeVisible();
  await expect(keyboard).toHaveAttribute('aria-label', 'Teclado flotante');
  await expect(keyboard.locator('[data-key-action="hide"]')).toHaveAccessibleName('Ocultar');
});

test('can reopen the tutorial after completing first-run onboarding', async ({ context }) => {
  const page = await openNewTab(context);
  await expect(page.locator('#onboarding-panel')).toBeHidden();
  await expect
    .poll(() =>
      page.evaluate(
        async () => (await chrome.storage.local.get('onboardingCompleted')).onboardingCompleted,
      ),
    )
    .toBe(true);

  await page.locator('#help-link').click();
  await expect(page.locator('#onboarding-panel')).toBeVisible();
  await page.locator('#onboarding-skip').click();
  await expect(page.locator('#onboarding-panel')).toBeHidden();

  const nextTab = await openNewTab(context);
  await expect(nextTab.locator('#onboarding-panel')).toBeHidden();
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

test('centers the launcher and shortcuts in one panel', async ({ context }) => {
  const page = await openNewTab(context);

  await expect(page.locator('.home-panel')).toBeVisible();
  await expect(page.getByRole('textbox', { name: 'Website address' })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Launch A', exact: true })).toBeVisible();
  await expect(page.getByRole('button', { name: 'Add shortcut' })).toBeVisible();
  await expect(page.locator('.keyboard-panel')).toHaveCount(0);

  const panelLayout = await page.locator('.home-panel').evaluate((panel) => {
    const rect = panel.getBoundingClientRect();
    return {
      left: rect.left,
      width: rect.width,
      viewport: window.innerWidth,
      formTop: panel.querySelector('#site-form')!.getBoundingClientRect().top,
      shortcutsTop: panel.querySelector('#pin-grid')!.getBoundingClientRect().top,
    };
  });
  expect(
    Math.abs(panelLayout.left + panelLayout.width / 2 - panelLayout.viewport / 2),
  ).toBeLessThan(2);
  expect(panelLayout.formTop).toBeLessThan(panelLayout.shortcutsTop);

  const pageHeight = await page.evaluate(() => ({
    content: document.documentElement.scrollHeight,
    viewport: window.innerHeight,
  }));
  expect(pageHeight.content).toBeLessThanOrEqual(pageHeight.viewport);
});

test('gamepad reaches the address field from the focused shortcut', async ({ context }) => {
  const page = await openNewTab(context);

  await sendFrame(page, makeFrame(), true);
  await sendFrame(page, makeFrame([12]));
  await expect(page.locator('#site-url')).toBeFocused();
});

test('gamepad types in the floating keyboard on the new tab', async ({ context }) => {
  test.setTimeout(60000);
  const page = await openNewTab(context);
  const status = page.locator('#controller-status');
  const keyboard = page.locator('[data-gamepad-navigator="keyboard"]');
  const urlShortcuts = keyboard.locator('[data-url-shortcuts]');

  await sendFrame(page, makeFrame(), true);
  await expect(status).toContainText('Controller ready');
  await sendFrame(page, makeFrame([15]));
  await expect(page.locator('[data-launch="youtube"]')).toBeFocused();
  await sendFrame(page, makeFrame(), true);

  await page.locator('#site-url').focus();
  await expect(keyboard).toBeVisible();
  await expect(urlShortcuts).toBeVisible();
  const siteUrl = page.locator('#site-url');
  await siteUrl.fill('example');
  await urlShortcuts.getByRole('button', { name: '.com' }).click();
  await expect(siteUrl).toHaveValue('example.com');
  await siteUrl.evaluate((input) => (input as HTMLInputElement).setSelectionRange(0, 0));
  await urlShortcuts.getByRole('button', { name: 'https://' }).click();
  await expect(siteUrl).toHaveValue('https://example.com');
  await keyboard.locator('[data-key-action="clear"]').click();
  await expect(siteUrl).toHaveValue('');

  await sendFrame(page, makeFrame([13]));
  await expect(keyboard.locator('[data-key="1"]')).toBeFocused();
  await sendFrame(page, makeFrame(), true);
  await keyboard.locator('[data-key="q"]').focus();
  await sendFrame(page, makeFrame([0]));
  await expect(page.locator('#site-url')).toHaveValue('q');
  await sendFrame(page, makeFrame(), true);

  await keyboard.locator('[data-key="."]').focus();
  await sendFrame(page, makeFrame([0]));
  await expect(page.locator('#site-url')).toHaveValue('q.');
  await sendFrame(page, makeFrame(), true);

  await keyboard.locator('[data-key-action="backspace"]').focus();
  await sendFrame(page, makeFrame([0]));
  await expect(page.locator('#site-url')).toHaveValue('q');
  await sendFrame(page, makeFrame(), true);

  await keyboard.locator('[data-key-action="shift"]').focus();
  await sendFrame(page, makeFrame([0]));
  await expect(keyboard.locator('[data-key-action="shift"]')).toHaveAttribute(
    'aria-pressed',
    'true',
  );
  await sendFrame(page, makeFrame(), true);
  await keyboard.locator('[data-key="z"]').focus();
  await sendFrame(page, makeFrame([0]));
  await expect(page.locator('#site-url')).toHaveValue('qZ');
  await sendFrame(page, makeFrame(), true);

  await keyboard.locator('[data-key-action="clear"]').focus();
  await sendFrame(page, makeFrame([0]));
  await expect(page.locator('#site-url')).toHaveValue('');
  await page.locator('#site-label').focus();
  await expect(keyboard).toBeVisible();
  await expect(urlShortcuts).toBeHidden();
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

test('R3 opens a new navigator tab from the new tab page', async ({ context }) => {
  const navigatorPage = await openNewTab(context);
  const initialCount = context.pages().length;
  const newPagePromise = context.waitForEvent('page');

  await sendFrame(navigatorPage, makeFrame(), true);
  await sendFrame(navigatorPage, makeFrame([11]));

  const newPage = await newPagePromise;
  await expect.poll(() => context.pages().length).toBe(initialCount + 1);
  await expect(newPage).toHaveURL(/chrome-extension:\/\/[^/]+\/newtab\.html/);
  await expect(newPage.getByRole('heading', { name: /choose your next world/i })).toBeVisible();
});

test('RB + X refreshes the new tab page', async ({ context }) => {
  const navigatorPage = await openNewTab(context);
  await sendFrame(navigatorPage, makeFrame(), true);

  const reload = navigatorPage.waitForEvent('load');
  await sendFrame(navigatorPage, makeFrame([5]), true);
  await sendFrame(navigatorPage, makeFrame([2, 5]));
  await reload;

  await expect(navigatorPage).toHaveURL(/chrome-extension:\/\/[^/]+\/newtab\.html/);
  await expect(
    navigatorPage.getByRole('heading', { name: /choose your next world/i }),
  ).toBeVisible();
});

test('shoulder navigation keeps working across repeated tab switches', async ({ context }) => {
  const navigatorPage = await openNewTab(context);
  for (const page of context.pages()) {
    if (page !== navigatorPage) await page.close();
  }

  const firstPage = await context.newPage();
  await firstPage.goto('https://alpha.test/first');
  const secondPage = await context.newPage();
  await secondPage.goto('https://alpha.test/second');
  await navigatorPage.bringToFront();

  const pages = [navigatorPage, firstPage, secondPage];
  let currentIndex = 0;
  for (const button of [5, 5, 5, 4, 4, 5, 4, 5]) {
    const nextIndex = (currentIndex + (button === 5 ? 1 : -1) + pages.length) % pages.length;
    await sendFrame(pages[currentIndex], makeFrame(), true);
    await sendFrame(pages[currentIndex], makeFrame([button]));
    await expect
      .poll(() => pages[nextIndex].evaluate(() => document.visibilityState))
      .toBe('visible');
    currentIndex = nextIndex;
  }
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

test('renders Chromium’s favicon fallback before using the cached site favicon', async ({
  context,
}) => {
  const page = await openNewTab(context);
  await page.locator('#site-url').fill('https://alpha.test/arcade');
  await page.locator('#site-label').fill('Alpha Arcade');
  await page.getByRole('button', { name: 'Add shortcut' }).click();

  let tile = page.getByRole('button', { name: 'Open Alpha Arcade, alpha.test' });
  await expect(tile.locator('.tile-favicon')).toBeVisible();
  await expect(tile.locator('.tile-favicon')).toHaveJSProperty('naturalWidth', 32);

  const faviconResponse = page.waitForResponse('https://alpha.test/favicon.svg');
  await tile.click();
  await expect(page).toHaveURL('https://alpha.test/arcade');
  await faviconResponse;
  await page.goBack();
  await page.reload();

  tile = page.getByRole('button', { name: 'Open Alpha Arcade, alpha.test' });
  await expect(tile.locator('.tile-favicon')).toBeVisible();
});
