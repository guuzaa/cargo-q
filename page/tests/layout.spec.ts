import { expect, test, type Page } from '@playwright/test';
import { open } from './helpers';

/** How far down the page a section starts. */
async function top(page: Page, selector: string): Promise<number> {
  const box = await page.locator(selector).boundingBox();
  return box?.y ?? 0;
}

const FLAGS = 'section[aria-labelledby="flags-title"]';

test('on a wide screen the flags come first', async ({ page }) => {
  // given: a wide screen
  await page.setViewportSize({ width: 1280, height: 800 });
  // when: the page opens
  await open(page);
  // then: the flags come before the terminal
  expect(await top(page, FLAGS)).toBeLessThan(await top(page, '#playground'));
});

test('on a phone the terminal comes first', async ({ page }) => {
  // given: a phone
  await page.setViewportSize({ width: 390, height: 844 });
  // when: the page opens
  await open(page);
  // then: the terminal comes before the flags
  expect(await top(page, '#playground')).toBeLessThan(await top(page, FLAGS));
});
