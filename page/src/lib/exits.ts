/**
 * The exit codes the page promises.
 *
 * The table of them and the test that says every one of them is the code a
 * run leaves read the same list, so the promise and the run cannot drift
 * apart.
 */

export interface ExitRow {
  /** The line that is run. */
  line: string;
  /** The code the run leaves behind. */
  code: number;
  kind: 'ok' | 'fail' | 'int';
  /** The key that has to be pressed, for a run that never ends. */
  key?: string;
}

export const exits: ExitRow[] = [
  { line: 'cargo q fmt check', code: 0, kind: 'ok' },
  { line: 'cargo q check test build', code: 101, kind: 'fail' },
  { line: 'cargo q build --features feature1', code: 1, kind: 'fail' },
  { line: 'cargo q -p --repeat 2 check', code: 2, kind: 'fail' },
  { line: 'cargo q watch', code: 130, kind: 'int', key: '^C' },
];
