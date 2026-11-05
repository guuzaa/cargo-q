import { expect, test, type Page } from '@playwright/test';
import { open, over, run } from './helpers';

/** Wait until every command of the run has passed. */
async function allPassed(page: Page): Promise<void> {
  const pills = page.locator('#split .pill-command');
  await expect(pills).not.toHaveCount(0);
  await expect(page.locator('#split .pill-command[data-state="ok"]')).toHaveCount(await pills.count());
}

test.beforeEach(async ({ page }) => {
  await open(page);
});

test('every exit code the page promises is the code the run leaves', async ({ page }) => {
  const rows = page.locator('[data-reroll]');
  const count = await rows.count();
  expect(count).toBeGreaterThan(0);

  for (let index = 0; index < count; index += 1) {
    const row = rows.nth(index);
    const line = await row.getAttribute('data-run');
    const code = await row.locator('[data-odometer]').getAttribute('data-odometer');
    expect(line).toBeTruthy();
    expect(code).toBeTruthy();

    await row.click();

    if (code === '130') {
      // The run never ends on its own: it is stopped with ^C.
      await expect(page.locator('#split')).toHaveAttribute('data-mode', 'run');
      await page.keyboard.press('Control+c');
    }
    if (code === '0') {
      // A run that succeeds: its commands are marked as they pass.
      await allPassed(page);
    }
    await over(page);
    await expect(page.locator('#exit-code')).toHaveText(code!);
  }
});

test('the plan on a card is the plan cargo-q prints', async ({ page }) => {
  const card = page.locator('.flip').filter({ hasText: 'cargo q -n build --features=foo' });
  const plan = (await card.locator('pre').textContent()) ?? '';
  expect(plan).toContain('cargo build --features=foo');

  await run(page, 'cargo q -n build --features=foo');
  await expect(page.locator('#exit-code')).toHaveText('0');
  await expect(page.locator('#terminal .xterm-rows')).toContainText('cargo build --features=foo');
});

test('a chip runs the line it shows', async ({ page }) => {
  await page.locator('.chip[data-run="cargo q fmt check"]').click();
  await expect(page.locator('#split')).toHaveAttribute('data-mode', 'run');
  await expect(page.locator('#split .pill-command')).toHaveCount(2);
});
