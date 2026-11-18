import { expect, test } from '@playwright/test';
import { open } from './utils';

test('the page plays its first line when the reader scrolls to it', async ({ page }) => {
  // given: the page is open, and nothing has run yet
  await open(page);
  await expect(page.locator('#split')).not.toHaveAttribute('data-mode', 'run');

  // when: the reader scrolls to the terminal
  await page.evaluate(() => document.querySelector('#terminal')?.scrollIntoView({ block: 'center' }));

  // then: the first line plays
  await expect(page.locator('#split')).toHaveAttribute('data-mode', 'run', { timeout: 30_000 });
  await expect(page.locator('#split .pill-command')).toHaveCount(2);
});

test.describe('on a phone', () => {
  test.use({ viewport: { width: 390, height: 844 }, hasTouch: true, isMobile: true });

  test('the terminal plays its first line on a phone too', async ({ page }) => {
    // given: a phone, and a page that is open
    await open(page);
    // The touch that scrolls the page starts on the terminal: it is a
    // pointerdown, not a tap, and it must not look like someone typing.
    await page.locator('#terminal').dispatchEvent('pointerdown', { pointerType: 'touch' });

    // when: the reader scrolls to the terminal
    await page.evaluate(() => document.querySelector('#terminal')?.scrollIntoView({ block: 'center' }));

    // then: the first line plays
    await expect(page.locator('#split')).toHaveAttribute('data-mode', 'run', { timeout: 30_000 });
  });
});
