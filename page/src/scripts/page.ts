/**
 * The page's own wiring: the shell, the split under it, and the events
 * that run from one to the other.
 *
 * The terminal script and the tests use the same wiring, so what a test
 * says is about the page and not about a copy of it.
 */

import type { Terminal } from '@xterm/xterm';
import { Shell } from './shell';
import { SplitView } from './split-view';
import type { Parser } from './wasm';

export interface PageParts {
  /** The split under the terminal. */
  split: HTMLElement;
  /** Where the round is written. */
  status: HTMLElement;
  /** The quiet region the page says the line in. */
  live: HTMLElement;
}

export interface PageHooks {
  /** A run left a code behind. */
  exit(code: number, cargoQ: boolean): void;
}

export interface Page {
  shell: Shell;
  view: SplitView;
}

/** The shell and the split, wired to each other. */
export function connect(
  term: Terminal,
  parser: Promise<Parser>,
  parts: PageParts,
  hooks: PageHooks,
  suggestions: string[],
): Page {
  const view = new SplitView(parts.split, parts.status, parts.live);
  const shell = new Shell(
    term,
    parser,
    {
      input: (analysis) => view.input(analysis),
      run: (inspection) => view.run(inspection),
      task: (index, state, round) => view.task(index, state, round),
      exit: (code, cargoQ) => {
        hooks.exit(code, cargoQ);
        if (cargoQ) view.done();
      },
    },
    suggestions,
  );
  return { shell, view };
}
