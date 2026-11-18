import { expect, test, type Page } from '@playwright/test';
import { open } from './utils';

/** How far down the page a section starts. */
async function top(page: Page, selector: string): Promise<number> {
  const box = await page.locator(selector).boundingBox();
  return box?.y ?? 0;
}

/** How wide an element is. */
async function width(page: Page, selector: string): Promise<number> {
  const box = await page.locator(selector).boundingBox();
  return box?.width ?? 0;
}

const FLAGS = 'section[aria-labelledby="flags-title"]';

test('the terminal comes before the flags', async ({ page }) => {
  // given: a wide screen
  await page.setViewportSize({ width: 1280, height: 800 });
  // when: the page opens
  await open(page);
  // then: the terminal comes before the flags
  expect(await top(page, '#playground')).toBeLessThan(await top(page, FLAGS));
});

test('the flags take the width of the page', async ({ page }) => {
  // given: a wide screen
  await page.setViewportSize({ width: 1280, height: 800 });
  // when: the page opens
  await open(page);
  // then: the flags are as wide as the terminal
  expect(await width(page, FLAGS)).toBe(await width(page, '#playground'));
});

test('on a phone the flag cards are not crushed together', async ({ page }) => {
  // given: a phone
  await page.setViewportSize({ width: 390, height: 844 });
  // when: the page opens
  await open(page);
  // then: every flag card is wide enough to read
  const cards = page.locator(`${FLAGS} .flip`);
  const count = await cards.count();
  expect(count).toBe(6);
  for (let i = 0; i < count; i += 1) {
    const box = await cards.nth(i).boundingBox();
    expect(box?.width ?? 0).toBeGreaterThanOrEqual(300);
  }
});
