import { expect, test } from '@playwright/test';
import { open } from './utils';

test('the page is the landing page of cargo-q', async ({ page }) => {
  // given: the page is open
  await open(page);
  // when: the reader looks at it
  // then: it is the landing page of cargo-q
  await expect(page).toHaveTitle(/cargo-q/);
  await expect(page.locator('h1')).toContainText('Run Cargo only once');
});

test('the parser is loaded', async ({ page }) => {
  // given: the page is open
  await open(page);
  // when: the parser has had its say
  // then: the mark says the parser is there
  const badge = page.locator('#wasm-badge');
  await expect(badge).toHaveAttribute('data-ready', '');
  await expect(badge).toHaveAttribute('title', /cargo-q .* parser, compiled to WebAssembly/);
});

test('the page has no errors', async ({ page }) => {
  // given: a page that listens to the console
  const errors: string[] = [];
  page.on('console', (message) => {
    if (message.type() === 'error') errors.push(message.text());
  });
  page.on('pageerror', (error) => errors.push(error.message));

  // when: the reader clicks into the terminal
  await open(page);
  await page.locator('#terminal').click();
  await expect(page.locator('#split')).toHaveAttribute('data-mode', 'edit');

  // then: nothing was said in error
  expect(errors).toEqual([]);
});

test('the terminal paints itself', async ({ page }) => {
  // given: the page is open
  await open(page);
  // when: the terminal is on the screen
  await page.locator('#terminal').scrollIntoViewIfNeeded();
  // then: it shows what the terminal says
  await expect(page.locator('#terminal .xterm-rows')).toContainText('cargo-q');
});

test('the address that is wrong says so', async ({ page }) => {
  // given: an address that is wrong
  const address = '/nowhere';
  // when: it is asked for
  await page.goto(address);
  // then: the page says there is no such page
  await expect(page.locator('h1')).toContainText('No such page');
});
