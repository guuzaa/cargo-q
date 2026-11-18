/**
 * What the tests stand in for the browser.
 *
 * The page is a terminal and a split: the terminal is xterm.js, which paints
 * a screen out of the bytes it is given, and the tests care about the bytes
 * rather than the pixels. The stand-in keeps them.
 *
 * Nothing in the page imports this: it is here because the tests that sit
 * next to the code need it.
 */

import type { Terminal } from '@xterm/xterm';
import { vi } from 'vitest';
import { buildParser } from '../lib/parser';
import { Shell } from './shell';
import { SplitView } from './split-view';
import type { Parser } from './wasm';

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

export interface Page {
  shell: Shell;
  view: SplitView;
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

/** The page's own wiring: a shell, a split, and the terminal between them. */
export function makePage(): Page {
  const root = document.createElement('div');
  const status = document.createElement('div');
  const live = document.createElement('p');
  const view = new SplitView(root, status, live);
  const terminal = new FakeTerminal();
  const codes: number[] = [];
  const waiters: ((code: number) => void)[] = [];

  const shell = new Shell(
    terminal as unknown as Terminal,
    parser(),
    {
      input: (analysis) => view.input(analysis),
      run: (inspection) => view.run(inspection),
      task: (index, state, round) => view.task(index, state, round),
      exit: (code) => {
        view.done();
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
export async function type(page: Page, line: string, options: { run?: boolean } = {}): Promise<void> {
  const typing = page.shell.type(line, { instant: true, ...options });
  await vi.advanceTimersByTimeAsync(1);
  await typing;
}

/** Let the fake clock run: the simulation lives on it. */
export async function settle(ms = 60_000): Promise<void> {
  await vi.advanceTimersByTimeAsync(ms);
}

/** The commands the split shows, one by one. */
export function commands(page: Page): string[] {
  return [...page.root.querySelectorAll('.pill-command')].map((pill) =>
    (pill.textContent ?? '').replace(/\s+/g, ' ').trim(),
  );
}

/** The state of every command of the split, in order. */
export function states(page: Page): (string | null)[] {
  return [...page.root.querySelectorAll('.pill-command')].map((pill) => pill.getAttribute('data-state'));
}
