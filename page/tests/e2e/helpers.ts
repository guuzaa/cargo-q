/**
 * The terminal on the page, from the outside.
 *
 * The terminal is an xterm.js terminal: what it shows is a screenful of
 * rows of text, and only those rows are in the page — the scrollback is
 * not. The tests therefore read what the page shows around the terminal
 * (the exit code, the pills of the split) and leave the text of the
 * terminal to the few places where it is the only thing to look at.
 */

import { expect, type Page } from '@playwright/test';

/** Open the page, and wait for the parser to be there. */
export async function open(page: Page): Promise<void> {
  await page.goto('/');
  // The badge is hidden on a phone, so wait for the mark, not for the box.
  await page.locator('#wasm-badge[data-ready]').waitFor({ state: 'attached' });
}

/**
 * Take the terminal.
 *
 * A terminal that has the focus is a terminal someone is using: the page
 * does not play its first line then. Most tests want that.
 */
export async function focus(page: Page): Promise<void> {
  await page.locator('#terminal').click();
}

/** Type a line into the terminal, and run it. */
export async function run(page: Page, line: string): Promise<void> {
  await page.locator('#terminal').click();
  await page.keyboard.type(line);
  await page.keyboard.press('Enter');
}

/** Wait until the run in the terminal is over. */
export async function over(page: Page): Promise<void> {
  await expect(page.locator('#split .pill:not(.leaving)[data-state="running"]')).toHaveCount(0);
  await expect(page.locator('#split .pill:not(.leaving)[data-state="pending"]')).toHaveCount(0);
}

/** The commands the split shows, one by one. */
export async function commands(page: Page): Promise<string[]> {
  const pills = page.locator('#split .pill-command');
  const texts: string[] = [];
  for (let index = 0; index < (await pills.count()); index += 1) {
    texts.push((await pills.nth(index).innerText()).replace(/\s+/g, ' ').trim());
  }
  return texts;
}
