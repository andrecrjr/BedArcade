import type { Page } from '@playwright/test';
import { E2E_GAMEPAD_MESSAGE, type GamepadFrame } from '../../lib/gamepad';
import { expect, test } from './fixtures';

function makeFrame(pressed: number[] = []): GamepadFrame {
  return {
    id: 'E2E standard gamepad',
    connected: true,
    axes: [0, 0, 0, 0],
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

test('configures gamepad hotkeys from the new tab and applies saved bindings', async ({
  context,
  page,
}) => {
  await context.route('https://www.youtube.com/**', (route) =>
    route.fulfill({
      status: 200,
      contentType: 'text/html',
      body: '<!doctype html><button>Play video</button>',
    }),
  );
  await context.route('https://alpha.test/**', (route) =>
    route.fulfill({
      status: 200,
      contentType: 'text/html',
      body: '<!doctype html><button id="confirm" onclick="document.body.dataset.activated = \'confirm\'">Confirm</button>',
    }),
  );
  await page.goto('chrome://newtab/');
  await expect(page).toHaveURL(/chrome-extension:\/\/[^/]+\/newtab\.html/);

  await page.getByRole('link', { name: 'Controller settings' }).click();
  await expect(page).toHaveURL(/chrome-extension:\/\/[^/]+\/settings\.html/);
  await expect(page.getByRole('heading', { name: 'Controller hotkeys' })).toBeVisible();
  await expect(page.locator('[data-binding]')).toHaveCount(12);
  await expect(page.locator('#binding-activate')).toHaveValue('0');
  await expect(page.locator('#binding-refresh-modifier')).toHaveValue('5');
  await expect(page.locator('#binding-refresh-trigger')).toHaveValue('2');

  await page.locator('#binding-activate').selectOption('3');
  await page.getByRole('button', { name: 'Save controls' }).click();
  await expect(page.locator('#settings-feedback')).toHaveText('Controls saved.');
  const savedBindings = await page.evaluate(
    async () => (await chrome.storage.local.get('gamepadBindings')).gamepadBindings,
  );
  expect(savedBindings).toMatchObject({ activate: 3 });

  const newTabUrl = await page.evaluate(() => chrome.runtime.getURL('newtab.html'));
  await page.goto(newTabUrl);
  await expect(page.getByRole('heading', { name: /choose your next world/i })).toBeVisible();
  await page.locator('[data-launch="youtube"]').focus();
  await sendFrame(page, makeFrame(), true);
  await sendFrame(page, makeFrame([3]));
  await expect(page).toHaveURL('https://www.youtube.com/');

  await page.goto('https://alpha.test/controls');
  await expect(page.locator('[data-gamepad-navigator="status"]')).toBeAttached();
  await sendFrame(page, makeFrame(), true);
  await page.locator('#confirm').focus();
  await sendFrame(page, makeFrame([3]));
  await expect(page.locator('body')).toHaveAttribute('data-activated', 'confirm');
});
