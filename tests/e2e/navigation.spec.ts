import type { BrowserContext, Page } from '@playwright/test';
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

const textFixtureHtml = `<!doctype html>
<html>
  <body>
    <input id="text-input" type="text" value="ab" />
    <textarea id="message"></textarea>
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

async function sendTabActions(page: Page, action: 'prev-tab' | 'next-tab', count = 1) {
  await page.evaluate(
    ({ nextAction, nextCount }) => {
      for (let index = 0; index < nextCount; index += 1)
        window.postMessage({ type: 'gamepad-navigator:e2e-tab-action', action: nextAction }, '*');
    },
    { nextAction: action, nextCount: count },
  );
}

async function currentTabOrder(context: BrowserContext) {
  const worker = context.serviceWorkers()[0];
  if (!worker) throw new Error('Extension service worker is not running');
  return worker.evaluate(async () =>
    (await chrome.tabs.query({ currentWindow: true })).map((tab) => ({
      active: Boolean(tab.active),
      index: tab.index ?? -1,
    })),
  );
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
  await context.route(/^https?:\/\/(alpha|beta)\.test\//, (route) => {
    const isTextFixture = new URL(route.request().url()).pathname === '/text-fields';
    return route.fulfill({
      status: 200,
      contentType: 'text/html',
      body: isTextFixture ? textFixtureHtml : fixtureHtml,
    });
  });
});

test('shows a floating keyboard for text inputs and textareas', async ({ page }) => {
  await page.goto('https://alpha.test/text-fields');
  await connectPad(page);

  const keyboard = page.locator('[data-gamepad-navigator="keyboard"]');
  const textInput = page.locator('#text-input');
  await expect(keyboard.locator('[data-url-shortcuts]')).toHaveCount(0);
  await textInput.evaluate((input) => (input as HTMLInputElement).setSelectionRange(1, 1));
  await textInput.focus();
  await expect(keyboard).toBeVisible();
  await expect
    .poll(() =>
      keyboard.evaluate((element) => {
        const field = document.querySelector('#text-input');
        if (!field) throw new Error('Text input is missing');
        const keyboardRect = element.getBoundingClientRect();
        const fieldRect = field.getBoundingClientRect();
        return Math.min(
          Math.abs(keyboardRect.bottom - fieldRect.top),
          Math.abs(keyboardRect.top - fieldRect.bottom),
        );
      }),
    )
    .toBeLessThanOrEqual(16);
  await textInput.click();
  await expect(keyboard).toBeVisible();
  await textInput.evaluate((input) => (input as HTMLInputElement).setSelectionRange(1, 1));
  await keyboard.locator('[data-key="x"]').focus();
  await sendFrame(page, makeFrame([0]));
  await expect(textInput).toHaveValue('axb');
  await sendFrame(page, makeFrame(), true);

  const textarea = page.locator('#message');
  await textarea.focus();
  await expect(textarea).toBeFocused();
  await expect(keyboard).toBeVisible();
  await expect
    .poll(() =>
      keyboard.evaluate((element) => {
        const field = document.querySelector('#message');
        if (!field) throw new Error('Textarea is missing');
        const keyboardRect = element.getBoundingClientRect();
        const fieldRect = field.getBoundingClientRect();
        return Math.min(
          Math.abs(keyboardRect.bottom - fieldRect.top),
          Math.abs(keyboardRect.top - fieldRect.bottom),
        );
      }),
    )
    .toBeLessThanOrEqual(16);
  await keyboard.locator('[data-key="y"]').focus();
  await sendFrame(page, makeFrame([0]));
  await expect(textarea).toHaveValue('y');
  await page.locator('[data-gamepad-navigator="keyboard"] [data-key="z"]').click();
  await expect(textarea).toHaveValue('yz');
  await page.locator('[data-gamepad-navigator="keyboard"] [data-key-action="enter"]').click();
  await expect(textarea).toHaveValue('yz\n');
});

test('keeps directional focus inside the floating keyboard', async ({ page }) => {
  await page.goto('https://alpha.test/text-fields');
  await connectPad(page);

  const keyboard = page.locator('[data-gamepad-navigator="keyboard"]');
  await page.locator('#text-input').focus();
  await expect(keyboard).toBeVisible();
  await keyboard.locator('[data-key="1"]').focus();
  await sendFrame(page, makeFrame([12]), true);

  await expect
    .poll(() =>
      page.evaluate(() =>
        Boolean(document.activeElement?.closest('[data-gamepad-navigator="keyboard"]')),
      ),
    )
    .toBe(true);
});

test('hiding the floating keyboard returns focus to its text field', async ({ page }) => {
  await page.goto('https://alpha.test/text-fields');
  await connectPad(page);

  const keyboard = page.locator('[data-gamepad-navigator="keyboard"]');
  const textInput = page.locator('#text-input');
  await textInput.focus();
  await expect(keyboard).toBeVisible();
  await keyboard.locator('[data-key-action="hide"]').focus();
  await sendFrame(page, makeFrame([0]));

  await expect(keyboard).toBeHidden();
  await expect(textInput).toBeFocused();
});

test('rapid tab presses continue from the newly active tab', async ({ context, page }) => {
  await page.goto('https://alpha.test/controls');
  const firstTab = await context.newPage();
  await firstTab.goto('http://beta.test/first');
  const secondTab = await context.newPage();
  await secondTab.goto('https://alpha.test/second');
  await page.bringToFront();
  const initialTabs = await currentTabOrder(context);
  const startIndex = initialTabs.find((tab) => tab.active)?.index ?? -1;
  expect(initialTabs.length).toBeGreaterThanOrEqual(3);
  expect(startIndex).toBeGreaterThanOrEqual(0);

  await sendTabActions(page, 'next-tab');
  await expect
    .poll(async () => (await currentTabOrder(context)).find((tab) => tab.active)?.index)
    .toBe((startIndex + 1) % initialTabs.length);
  await sendTabActions(page, 'next-tab', 2);
  await expect
    .poll(async () => (await currentTabOrder(context)).find((tab) => tab.active)?.index)
    .toBe((startIndex + 3) % initialTabs.length);
});

test('R3 opens a new navigator tab from a website', async ({ context, page }) => {
  await page.goto('https://alpha.test/controls');
  await connectPad(page);
  const initialCount = context.pages().length;

  await sendFrame(page, makeFrame([11]));

  await expect.poll(() => context.pages().length).toBe(initialCount + 1);
  const newPage = context.pages().find((current) => current !== page);
  if (!newPage) throw new Error('The R3 action did not create a tab');
  await expect(newPage).toHaveURL(/chrome-extension:\/\/[^/]+\/newtab\.html/);
  await expect.poll(() => newPage.evaluate(() => document.visibilityState)).toBe('visible');
});

test('RB + X refreshes the current website without switching tabs', async ({ context, page }) => {
  await page.goto('https://alpha.test/controls');
  await connectPad(page);
  const otherPage = await context.newPage();
  await otherPage.goto('https://beta.test/second');
  await page.bringToFront();
  const initialIndex = (await currentTabOrder(context)).find((tab) => tab.active)?.index ?? -1;

  const reload = page.waitForEvent('load');
  await sendFrame(page, makeFrame([5]), true);
  await sendFrame(page, makeFrame([2, 5]));
  await reload;

  await expect(page).toHaveURL('https://alpha.test/controls');
  await expect
    .poll(async () => (await currentTabOrder(context)).find((tab) => tab.active)?.index)
    .toBe(initialIndex);
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
