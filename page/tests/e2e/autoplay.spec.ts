import { expect, test } from '@playwright/test';
import { open } from './helpers';

test('the page plays its first line when the reader scrolls to it', async ({ page }) => {
  await open(page);
  // Nothing runs by itself: the page waits for the reader.
  await expect(page.locator('#split')).not.toHaveAttribute('data-mode', 'run');

  await page.evaluate(() => document.querySelector('#terminal')?.scrollIntoView({ block: 'center' }));

  await expect(page.locator('#split')).toHaveAttribute('data-mode', 'run', { timeout: 30_000 });
  await expect(page.locator('#split .pill-command')).toHaveCount(2);
});

test.describe('on a phone', () => {
  test.use({ viewport: { width: 390, height: 844 }, hasTouch: true, isMobile: true });

  test('the terminal plays its first line on a phone too', async ({ page }) => {
    await open(page);
    // A touch that scrolls the page starts on the terminal: it is a
    // pointerdown, not a tap, and it must not look like someone typing.
    await page.locator('#terminal').dispatchEvent('pointerdown', { pointerType: 'touch' });
    await page.evaluate(() => document.querySelector('#terminal')?.scrollIntoView({ block: 'center' }));

    await expect(page.locator('#split')).toHaveAttribute('data-mode', 'run', { timeout: 30_000 });
  });
});
