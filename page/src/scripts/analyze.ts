/**
 * What each word of a line is, according to cargo-q's own parser.
 *
 * The shell colors the line from this as it is typed, and the split view
 * under the terminal draws the commands the line would run.
 */

import type { Inspection, Parser } from './wasm';
import { segments, tokenize, type Token } from './tokens';

export type Role =
  | 'program' // the program a command starts with
  | 'sub' // the `q` that names cargo-q
  | 'option' // one of cargo-q's own options
  | 'command' // a word that starts a cargo command
  | 'arg' // an argument of the command before it
  | 'error' // the word cargo-q rejects
  | 'after' // whatever follows a rejected word
  | 'unknown' // a program the shell does not have
  | 'word' // an argument of some other program
  | 'op'; // `&&`, `||`, `;`

export interface Painted extends Token {
  role: Role;
}

/** One `cargo q …` of the line, as cargo-q sees it. */
export interface Invocation {
  /** The words of the line, as a shell passes them. */
  argv: string[];
  inspection: Inspection;
}

export interface Analysis {
  line: string;
  tokens: Painted[];
  open: boolean;
  /** The `cargo q` under the cursor, when there is one. */
  focus: Invocation | null;
}

/** The programs the simulated shell has. */
export const PROGRAMS = ['cargo', 'echo', 'clear', 'history', 'help', 'ls', 'cat', 'pwd', 'true', 'false', 'exit'];

/** Whether a rejection is cargo-q's "there is no command yet". */
export function isNoCommand(error: string | null): boolean {
  return error !== null && error.includes('required arguments were not provided');
}

export function analyze(line: string, cursor: number, parser: Parser | null): Analysis {
  const { tokens, open } = tokenize(line);
  const painted: Painted[] = [];
  let focus: Invocation | null = null;

  for (const { words } of segments(tokens)) {
    const { roles, inspection } = paintSegment(words, parser);
    words.forEach((word, i) => painted.push({ ...word, role: roles[i] }));

    if (isCargoQ(words) && parser && inspection) {
      const argv = words.map((word) => word.text);
      const first = words[0].start;
      const last = words[words.length - 1].end;
      const inside = cursor >= first && cursor <= last + 1;
      if (inside || !focus) focus = { argv, inspection };
    }
  }
  for (const token of tokens) if (token.op) painted.push({ ...token, role: 'op' });
  painted.sort((a, b) => a.start - b.start);
  return { line, tokens: painted, open, focus };
}

/** Whether a command starts with `cargo q`, the way cargo runs a subcommand. */
export function isCargoQ(words: { text: string }[]): boolean {
  return words[0]?.text === 'cargo' && words[1]?.text === 'q';
}

/** What one segment of the line is made of. */
interface PaintedSegment {
  /** The role of every word of the segment. */
  roles: Role[];
  /**
   * What cargo-q makes of the whole segment, when the segment is a
   * `cargo q` line. The words are inspected one prefix at a time, so the
   * last of those inspections is the whole segment: the caller that wants
   * it does not have to ask again.
   */
  inspection: Inspection | null;
}

function paintSegment(words: Token[], parser: Parser | null): PaintedSegment {
  if (words.length === 0) return { roles: [], inspection: null };
  const program = words[0].text;
  const roles: Role[] = words.map(() => 'word');
  roles[0] = PROGRAMS.includes(program) ? 'program' : 'unknown';
  if (program !== 'cargo' || words.length < 2) return { roles, inspection: null };

  if (words[1].text !== 'q') {
    // Plain cargo: the subcommand, then its arguments.
    const known = parser?.names.includes(words[1].text) ?? true;
    roles[1] = words[1].text.startsWith('-') ? 'arg' : known ? 'command' : 'error';
    for (let i = 2; i < words.length; i += 1) roles[i] = 'arg';
    return { roles, inspection: null };
  }

  roles[1] = 'sub';
  if (!parser) return { roles: roles.map((role, i) => (i > 1 ? 'arg' : role)), inspection: null };

  // Ask the parser about every prefix of the line. The word that makes the
  // number of commands grow starts a command, the word that makes the line
  // fail is the one cargo-q rejects, and a word that changes nothing is one
  // cargo-q drops (the `q` that names the subcommand).
  //
  // The words are gathered as the prefix grows, so the array is built once
  // instead of being copied for every prefix.
  const argv = words.slice(0, 2).map((word) => word.text);
  let previous = parser.inspect(argv);
  let commands = 0;
  // Words that failed, until a later word shows whether they were unfinished.
  const pending: number[] = [];
  // Words that changed nothing while the line was failing.
  const quiet: number[] = [];

  for (let i = 2; i < words.length; i += 1) {
    argv.push(words[i].text);
    const inspection = parser.inspect(argv);
    const unchanged = sameInspection(inspection, previous);
    previous = inspection;

    if (inspection.error && !isNoCommand(inspection.error)) {
      // It may only be unfinished (`--repeat` before its count): wait for
      // the words that follow to tell.
      (unchanged && pending.length > 0 ? quiet : pending).push(i);
      continue;
    }
    if (unchanged) {
      roles[i] = 'sub';
      continue;
    }
    const count = inspection.commands.length;
    const role: Role = count > commands ? 'command' : count === 0 ? 'option' : 'arg';
    for (const p of pending) roles[p] = count === 0 ? 'option' : 'arg';
    for (const q of quiet) roles[q] = 'sub';
    pending.length = 0;
    quiet.length = 0;
    roles[i] = role;
    commands = count;
  }

  pending.forEach((i, n) => {
    roles[i] = n === 0 ? 'error' : 'after';
  });
  for (const q of quiet) roles[q] = 'after';
  return { roles, inspection: previous };
}

/**
 * Whether two inspections say the same thing.
 *
 * The same test as comparing the two as JSON, without building the JSON:
 * the line is inspected once per word, and the answer decides whether a
 * word changed anything at all.
 */
function sameInspection(a: Inspection, b: Inspection): boolean {
  if (a === b) return true;
  if (
    a.exitCode !== b.exitCode ||
    a.help !== b.help ||
    a.version !== b.version ||
    a.error !== b.error ||
    a.plan !== b.plan
  ) {
    return false;
  }
  if (a.options.length !== b.options.length) return false;
  for (let i = 0; i < a.options.length; i += 1) if (a.options[i] !== b.options[i]) return false;
  if (a.commands.length !== b.commands.length) return false;
  for (let i = 0; i < a.commands.length; i += 1) {
    const left = a.commands[i];
    const right = b.commands[i];
    if (left.name !== right.name || left.args.length !== right.args.length) return false;
    for (let j = 0; j < left.args.length; j += 1) if (left.args[j] !== right.args[j]) return false;
  }
  const x = a.settings;
  const y = b.settings;
  return (
    x.verbose === y.verbose &&
    x.parallel === y.parallel &&
    x.keepGoing === y.keepGoing &&
    x.dryRun === y.dryRun &&
    x.noFancy === y.noFancy &&
    x.repeat === y.repeat
  );
}
