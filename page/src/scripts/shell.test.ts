import { afterEach, beforeAll, describe, expect, it, vi } from 'vitest';
import { commands, makePage, parser, settle, states, type, type Page } from './testing';

beforeAll(async () => {
  // The parser is a wasm module: it is loaded before the clock is faked.
  await parser();
});

afterEach(() => {
  vi.useRealTimers();
});

/** Type a line and let the run finish. */
async function run(page: Page, line: string): Promise<number> {
  const exited = page.nextExit();
  await type(page, line);
  await settle();
  return exited;
}

describe('the shell', () => {
  it('shows what cargo-q makes of the line', async () => {
    vi.useFakeTimers();
    const page = makePage();
    await type(page, 'cargo q fmt check');
    expect(commands(page)).toEqual(['cargo fmt', 'cargo check']);
  });

  it('leaves the code of the first failure behind', async () => {
    vi.useFakeTimers();
    const page = makePage();
    expect(await run(page, 'cargo q check test build')).toBe(101);
    expect(states(page)).toEqual(['ok', 'fail', 'skipped']);
  });

  it('reports a line cargo-q rejects', async () => {
    vi.useFakeTimers();
    const page = makePage();
    expect(await run(page, 'cargo q build --features feature1')).toBe(1);
    expect(page.root.querySelector('.pill-error')?.textContent).toContain('is not a cargo subcommand');
  });

  it('stops a run that never ends', async () => {
    vi.useFakeTimers();
    const page = makePage();
    const exited = page.nextExit();
    await type(page, 'cargo q watch');
    await settle(200);
    page.terminal.send('\x03');
    await settle();
    expect(await exited).toBe(130);
  });

  it('completes a word with the tab', async () => {
    vi.useFakeTimers();
    const page = makePage();
    await type(page, 'car', { run: false });
    page.terminal.send('\t');
    page.terminal.send('q fmt');
    page.terminal.send('\r');
    await settle();
    expect(commands(page)).toEqual(['cargo fmt']);
  });

  it('brings a line back with the history', async () => {
    vi.useFakeTimers();
    const page = makePage();
    await run(page, 'echo alpha');
    const before = page.terminal.text.length;
    page.terminal.send('\x1b[A');
    expect(page.terminal.text.slice(before)).toContain('echo alpha');
  });

  it('prints the help of cargo-q', async () => {
    vi.useFakeTimers();
    const page = makePage();
    expect(await run(page, 'cargo q -h')).toBe(0);
    expect(page.terminal.text).toContain('Usage: cargo-q [OPTIONS] <COMMANDS>...');
  });

  it('knows what it is', async () => {
    vi.useFakeTimers();
    const page = makePage();
    await run(page, 'pwd');
    expect(page.terminal.text).toContain('/home/you/cargo-q');
  });

  it('says what the line came to, once the typing stops', async () => {
    vi.useFakeTimers();
    const page = makePage();
    await type(page, 'cargo q fmt check', { run: false });
    expect(page.live.textContent).toBe('');
    await settle(800);
    expect(page.live.textContent).toBe('cargo q runs 2 commands: fmt, check');
  });
});
