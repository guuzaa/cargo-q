import { type Page } from '@playwright/test';

/** Open the page, and wait for the parser to be there. */
export async function open(page: Page): Promise<void> {
  await page.goto('/');
  // The badge is hidden on a phone, so wait for the mark, not for the box.
  await page.locator('#wasm-badge[data-ready]').waitFor({ state: 'attached' });
}
