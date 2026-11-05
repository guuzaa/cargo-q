import { expect, test } from '@playwright/test';
import { commands, focus, open, over, run } from './helpers';

test.beforeEach(async ({ page }) => {
  await open(page);
});

test('the split shows what cargo-q makes of the line', async ({ page }) => {
  await focus(page);
  await page.keyboard.type('cargo q fmt check');
  await expect(page.locator('#split .pill-command')).toHaveCount(2);
  expect(await commands(page)).toEqual(['cargo fmt', 'cargo check']);
});

test('a run leaves the code of its first failure behind', async ({ page }) => {
  await run(page, 'cargo q check test build');
  await over(page);
  await expect(page.locator('#exit-code')).toHaveText('101');
  await expect(page.locator('#exit-code')).toHaveAttribute('data-kind', 'fail');
});

test('the run stops at the first failure', async ({ page }) => {
  await run(page, 'cargo q check test build');
  await over(page);
  const pills = page.locator('#split .pill-command');
  await expect(pills).toHaveCount(3);
  await expect(pills.nth(0)).toHaveAttribute('data-state', 'ok');
  await expect(pills.nth(1)).toHaveAttribute('data-state', 'fail');
  await expect(pills.nth(2)).toHaveAttribute('data-state', 'skipped');
});

test('the exit code follows the run', async ({ page }) => {
  await run(page, 'cargo q check test build');
  await over(page);
  await expect(page.locator('#exit-code')).toHaveText('101');

  await run(page, 'cargo q fmt check');
  await over(page);
  await expect(page.locator('#exit-code')).toHaveText('0');
  await expect(page.locator('#exit-code')).toHaveAttribute('data-kind', 'ok');
});

test('a run that never ends can be stopped', async ({ page }) => {
  await run(page, 'cargo q watch');
  await expect(page.locator('#split .pill-command').first()).toHaveAttribute('data-state', 'running');
  await page.keyboard.press('Control+c');
  await over(page);
  await expect(page.locator('#exit-code')).toHaveText('130');
  await expect(page.locator('#exit-code')).toHaveAttribute('data-kind', 'int');
});

test('a line cargo-q rejects is reported', async ({ page }) => {
  await run(page, 'cargo q build --features feature1');
  await expect(page.locator('#exit-code')).toHaveText('1');
  await expect(page.locator('#split .pill-error:not(.leaving)')).toContainText('is not a cargo subcommand');
  await expect(page.locator('#terminal .xterm-rows')).toContainText('is not a cargo subcommand');
});

test('the help is the help of cargo-q', async ({ page }) => {
  // The help is longer than the terminal is tall: make the terminal tall.
  await page.addStyleTag({ content: '#terminal { height: 40rem !important; }' });
  await run(page, 'cargo q -h');
  await expect(page.locator('#exit-code')).toHaveText('0');
  await expect(page.locator('#terminal .xterm-rows')).toContainText('Usage: cargo-q [OPTIONS] <COMMANDS>...');
  await expect(page.locator('#terminal .xterm-rows')).toContainText('Run Cargo only once.');
});

test('the tab completes a word', async ({ page }) => {
  await focus(page);
  await page.keyboard.type('car');
  await page.keyboard.press('Tab');
  await page.keyboard.type('q fmt');
  await page.keyboard.press('Enter');
  await over(page);
  await expect(page.locator('#exit-code')).toHaveText('0');
  expect(await commands(page)).toEqual(['cargo fmt']);
});

test('the history brings a line back', async ({ page }) => {
  await run(page, 'echo alpha');
  await expect(page.locator('#terminal .xterm-rows')).toContainText(/^alpha$/m, { useInnerText: true });

  await page.keyboard.press('ArrowUp');
  await page.keyboard.type('X');
  await page.keyboard.press('Enter');
  await expect(page.locator('#terminal .xterm-rows')).toContainText(/^alphaX$/m, { useInnerText: true });
});

test('a command that is not a cargo subcommand is refused', async ({ page }) => {
  await run(page, 'cargo q check frobnicate');
  await expect(page.locator('#exit-code')).toHaveText('1');
  await expect(page.locator('#terminal .xterm-rows')).toContainText('is not a cargo subcommand');
});

test('the shell knows what it is', async ({ page }) => {
  await run(page, 'pwd');
  await expect(page.locator('#terminal .xterm-rows')).toContainText(/^\/home\/you\/cargo-q$/m, { useInnerText: true });
});

test('the exit code of a plain cargo command is its own', async ({ page }) => {
  await run(page, 'cargo check');
  await over(page);
  await expect(page.locator('#exit-code')).toHaveText('0');
});
