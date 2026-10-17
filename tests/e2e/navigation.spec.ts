import type { Page } from '@playwright/test';
import { E2E_GAMEPAD_MESSAGE, type GamepadFrame } from '../../lib/gamepad';
import { expect, test } from './fixtures';

const fixtureHtml = `<!doctype html>
<html>
  <head>
    <meta charset="utf-8">
    <style>
      button { position: fixed; width: 100px; height: 40px; }
      #first { left: 20px; top: 20px; }
      #east { left: 220px; top: 20px; }
      #south { left: 20px; top: 120px; }
    </style>
  </head>
  <body>
    <button id="first">First</button>
    <button id="east" onclick="document.body.dataset.activated = 'east'">East</button>
    <button id="south">South</button>
  </body>
</html>`;

function makeFrame(pressed: number[] = [], axes: number[] = [0, 0, 0, 0]): GamepadFrame {
  return {
    id: 'E2E standard gamepad',
    connected: true,
    axes,
    buttons: Array.from({ length: 16 }, (_, index) => ({ pressed: pressed.includes(index) })),
  };
}

async function sendFrame(page: Page, frame: GamepadFrame | null, waitForSample = false) {
  await page.evaluate(
    ({ type, frame: nextFrame }) => {
      window.postMessage({ type, frame: nextFrame }, '*');
    },
    { type: E2E_GAMEPAD_MESSAGE, frame },
  );
  if (waitForSample) {
    await page.evaluate(
      () =>
        new Promise<void>((resolve) => {
          requestAnimationFrame(() => requestAnimationFrame(() => resolve()));
        }),
    );
  }
}

async function connectPad(page: Page) {
  const status = page.locator('[data-gamepad-navigator="status"]');
  await expect(status).toBeAttached();
  await sendFrame(page, makeFrame(), true);
  await expect(status).toBeVisible();
  await expect(status).toHaveText('Gamepad ativo · Start: pausar');
  return status;
}

test.beforeEach(async ({ context }) => {
  await context.route(/^https?:\/\/(alpha|beta)\.test\//, (route) =>
    route.fulfill({
      status: 200,
      contentType: 'text/html',
      body: fixtureHtml,
    }),
  );
});

test('moves focus and activates controls across supported sites', async ({ page }) => {
  for (const url of ['https://alpha.test/controls', 'http://beta.test/controls']) {
    await page.goto(url);
    await connectPad(page);

    const axes = url.startsWith('https:') ? [0.8, 0, 0, 0] : [0, 0, 0, 0];
    await sendFrame(page, makeFrame(url.startsWith('https:') ? [] : [15], axes));
    await expect(page.locator('#east')).toBeFocused();
    await sendFrame(page, makeFrame(), true);

    await sendFrame(page, makeFrame([0]));
    await expect(page.locator('body')).toHaveAttribute('data-activated', 'east');
    await sendFrame(page, makeFrame(), true);
  }
});

test('moves immediately after a gamepad reconnects', async ({ page }) => {
  await page.goto('https://alpha.test/controls');
  const status = await connectPad(page);

  await sendFrame(page, makeFrame([15]));
  await expect(page.locator('#east')).toBeFocused();
  await sendFrame(page, null);
  await expect(status).toBeHidden();
  await sendFrame(page, makeFrame([13]), true);
  expect(await page.evaluate(() => document.activeElement?.id)).toBe('south');
});

test('Start blocks navigation and activation until resumed', async ({ page }) => {
  await page.goto('https://alpha.test/controls');
  const status = await connectPad(page);
  await sendFrame(page, makeFrame([15]));
  await expect(page.locator('#east')).toBeFocused();
  await sendFrame(page, makeFrame(), true);

  await sendFrame(page, makeFrame([9]));
  await expect(status).toHaveText('Gamepad pausado · Start: retomar');
  await sendFrame(page, makeFrame(), true);
  await sendFrame(page, makeFrame([13]), true);
  // Hold beyond the movement repeat delay to confirm pause suppresses navigation.
  await page.waitForTimeout(250);
  expect(await page.evaluate(() => document.activeElement?.id)).toBe('east');
  await sendFrame(page, makeFrame(), true);
  await sendFrame(page, makeFrame([0]), true);
  await expect(page.locator('body')).not.toHaveAttribute('data-activated', 'east');
  await sendFrame(page, makeFrame(), true);
  await sendFrame(page, makeFrame([9]));
  await expect(status).toHaveText('Gamepad ativo · Start: pausar');
  await sendFrame(page, makeFrame(), true);
  await sendFrame(page, makeFrame([0]));
  await expect(page.locator('body')).toHaveAttribute('data-activated', 'east');
});

test('B returns to the previous page', async ({ page }) => {
  await page.goto('https://alpha.test/first');
  await page.goto('https://alpha.test/second');
  await connectPad(page);

  await sendFrame(page, makeFrame([1]));
  await expect(page).toHaveURL('https://alpha.test/first');
});

test('LB and RB switch between browser tabs', async ({ context, page }) => {
  await page.goto('https://alpha.test/first');
  await connectPad(page);
  const secondPage = await context.newPage();
  await secondPage.goto('http://beta.test/second');
  await connectPad(secondPage);

  await sendFrame(secondPage, makeFrame([4]));
  await expect.poll(() => page.evaluate(() => document.visibilityState)).toBe('visible');
  await sendFrame(page, makeFrame([5]));
  await expect.poll(() => secondPage.evaluate(() => document.visibilityState)).toBe('visible');
});
