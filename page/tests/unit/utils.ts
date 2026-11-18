/**
 * What the tests stand in for the browser.
 *
 * The page is a terminal and a split: the terminal is xterm.js, which paints
 * a screen out of the bytes it is given, and the tests care about the bytes
 * rather than the pixels. The stand-in keeps them.
 *
 * The wiring is the page's own (`src/scripts/page.ts`), so what a test says
 * is about the page and not about a copy of it.
 */

import type { Terminal } from '@xterm/xterm';
import { vi } from 'vitest';
import { buildParser } from '../../src/lib/parser';
import { connect, type Page } from '../../src/scripts/page';
import type { Parser } from '../../src/scripts/wasm';

/** cargo-q's own parser, compiled to WebAssembly. */
export function parser(): Promise<Parser> {
  return buildParser();
}

/** Everything the terminal was told, without the escape sequences. */
export function strip(text: string): string {
  return text.replace(/\x1b\[[0-9;?]*[A-Za-z]/g, '').replace(/\r\n/g, '\n');
}

/** A terminal that keeps what it was told to write. */
export class FakeTerminal {
  readonly cols = 80;
  readonly rows = 24;
  private readonly chunks: string[] = [];
  private listener: (data: string) => void = () => {};

  onData(listener: (data: string) => void): void {
    this.listener = listener;
  }

  write(text: string): void {
    this.chunks.push(text);
  }

  focus(): void {}

  /** A key, as a person presses it. */
  send(data: string): void {
    this.listener(data);
  }

  /** Everything the terminal was told, without the escape sequences. */
  get text(): string {
    return strip(this.chunks.join(''));
  }
}

/** The page, and what the tests need to look at it with. */
export interface FakePage extends Page {
  /** The terminal that stands in for xterm.js. */
  terminal: FakeTerminal;
  /** The split the page draws under the terminal. */
  root: HTMLElement;
  /** The quiet region the page says the line in. */
  live: HTMLElement;
  /** The codes the page was given, in order. */
  codes: number[];
  /** The code of the next run. */
  nextExit(): Promise<number>;
}

/** The page, with the terminal stood in for. */
export function makePage(): FakePage {
  const root = document.createElement('div');
  const status = document.createElement('div');
  const live = document.createElement('p');
  const terminal = new FakeTerminal();
  const codes: number[] = [];
  const waiters: ((code: number) => void)[] = [];

  const { shell, view } = connect(
    terminal as unknown as Terminal,
    parser(),
    { split: root, status, live },
    {
      exit: (code) => {
        codes.push(code);
        waiters.shift()?.(code);
      },
    },
    ['cargo q --help'],
  );

  return {
    shell,
    view,
    terminal,
    root,
    live,
    codes,
    nextExit: () => new Promise<number>((resolve) => waiters.push(resolve)),
  };
}

/**
 * Type a line into the shell, as a person would.
 *
 * The clock is a fake one: the typing does not wait for the pauses a person
 * takes, and the run that follows is left to the test.
 */
export async function type(page: FakePage, line: string, options: { run?: boolean } = {}): Promise<void> {
  const typing = page.shell.type(line, { instant: true, ...options });
  await vi.advanceTimersByTimeAsync(1);
  await typing;
}

/** Let the fake clock run: the simulation lives on it. */
export async function settle(ms = 60_000): Promise<void> {
  await vi.advanceTimersByTimeAsync(ms);
}

/** The commands the split shows, one by one. */
export function commands(page: FakePage): string[] {
  return [...page.root.querySelectorAll('.pill-command')].map((pill) =>
    (pill.textContent ?? '').replace(/\s+/g, ' ').trim(),
  );
}

/** The state of every command of the split, in order. */
export function states(page: FakePage): (string | null)[] {
  return [...page.root.querySelectorAll('.pill-command')].map((pill) => pill.getAttribute('data-state'));
}
