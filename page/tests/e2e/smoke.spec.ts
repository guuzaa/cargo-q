import { expect, test } from '@playwright/test';
import { open } from './helpers';

test('the page is the landing page of cargo-q', async ({ page }) => {
  await open(page);
  await expect(page).toHaveTitle(/cargo-q/);
  await expect(page.locator('h1')).toContainText('Run Cargo only once');
});

test('the parser is loaded', async ({ page }) => {
  await open(page);
  const badge = page.locator('#wasm-badge');
  await expect(badge).toHaveAttribute('data-ready', '');
  await expect(badge).toHaveAttribute('title', /cargo-q .* parser, compiled to WebAssembly/);
});

test('the page has no errors', async ({ page }) => {
  const errors: string[] = [];
  page.on('console', (message) => {
    if (message.type() === 'error') errors.push(message.text());
  });
  page.on('pageerror', (error) => errors.push(error.message));
  await open(page);
  await page.locator('#terminal').click();
  await expect(page.locator('#split')).toHaveAttribute('data-mode', 'edit');
  expect(errors).toEqual([]);
});

test('the terminal is there', async ({ page }) => {
  await open(page);
  await expect(page.locator('#terminal')).toBeVisible();
  // The terminal paints itself only when it is on the screen.
  await page.locator('#terminal').scrollIntoViewIfNeeded();
  await expect(page.locator('#terminal .xterm-rows')).toContainText('cargo-q');
});

test('the exit code starts at zero', async ({ page }) => {
  await open(page);
  await expect(page.locator('#exit-code')).toHaveText('0');
});

test('the address that is wrong says so', async ({ page }) => {
  await page.goto('/nowhere');
  await expect(page.locator('h1')).toContainText('No such page');
});
