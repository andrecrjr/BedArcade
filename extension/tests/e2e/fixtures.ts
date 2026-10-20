import { chromium, expect, test as base, type BrowserContext, type Page } from '@playwright/test';
import { resolve } from 'node:path';

export const test = base.extend<{ context: BrowserContext; page: Page }>({
  context: async ({ browserName }, use) => {
    if (browserName !== 'chromium') throw new Error('E2E tests require Chromium');
    const extensionPath = resolve('.output/chrome-mv3-e2e');
    const context = await chromium.launchPersistentContext('', {
      channel: 'chromium',
      headless: true,
      ignoreHTTPSErrors: true,
      args: [`--disable-extensions-except=${extensionPath}`, `--load-extension=${extensionPath}`],
    });
    try {
      await expect
        .poll(() =>
          context
            .pages()
            .some((page) => /chrome-extension:\/\/[^/]+\/newtab\.html/.test(page.url())),
        )
        .toBe(true);
      const launcherPage = context.pages().find((page) => /newtab\.html/.test(page.url()));
      if (!launcherPage) throw new Error('The first-install launcher did not open');
      await expect(launcherPage.locator('html')).toHaveAttribute('data-onboarding-ready', 'true');
      await use(context);
    } finally {
      await context.close();
    }
  },
  page: async ({ context }, use) => {
    const page = context.pages()[0] ?? (await context.newPage());
    await use(page);
  },
});

export { expect } from '@playwright/test';
