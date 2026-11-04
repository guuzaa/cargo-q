/**
 * A rehearsal of a cargo-q run.
 *
 * Which commands run, and in which order, comes from cargo-q's own parser.
 * What is simulated is cargo itself: each command takes a while, prints what
 * cargo would print, and `cargo test` fails on purpose. The display follows
 * `src/progress/`: the overprinting status on a terminal, one line per
 * command with `--no-fancy` or `-v`, the failures, and the summary.
 */

import type { Command } from './wasm';

export interface RunOptions {
  parallel: boolean;
  keepGoing: boolean;
  /** The overprinting display and color: a terminal without `--no-fancy`. */
  fancy: boolean;
  verbose: boolean;
  repeat: number;
}

export type TaskState = 'running' | 'ok' | 'fail' | 'skipped';

/** What the simulation needs from whoever shows it. */
export interface Host {
  write(text: string): void;
  sleep(ms: number): Promise<void>;
  stopped(): boolean;
  cols(): number;
  /** A command of the list changed state. */
  task(index: number, state: TaskState, round: number): void;
}

const R = '\x1b[0m';
const BOLD = '\x1b[1m';
const GREEN = '\x1b[32m';
const RED = '\x1b[31m';
const YELLOW = '\x1b[33m';
const OK = '\x1b[1;32m';
const ERR = '\x1b[1;31m';

const TICK = 80;
const ROOT = '/home/you/cargo-q';

interface Script {
  /** How long it takes, in milliseconds. */
  duration: number;
  /** The exit code it leaves; 0 is a success. */
  code: number;
  /** What cargo prints, with cargo's colors. */
  lines: string[];
}

const verb = (word: string, rest: string) => `${OK}${word.padStart(12)}${R} ${rest}`;
const compiling = verb('Compiling', `cargo-q v0.1.0 (${ROOT})`);
const finished = (profile: string, ms: number) =>
  verb('Finished', `\`${profile}\` profile [${profile === 'release' || profile === 'bench' ? 'optimized' : 'unoptimized + debuginfo'}] target(s) in ${(ms / 1000).toFixed(2)}s`);

/** The options of cargo that take a value. */
const TAKES_VALUE = new Map([
  ['--features', '--features <FEATURES>'],
  ['-F', '--features <FEATURES>'],
  ['-p', '--package [<SPEC>]'],
  ['--package', '--package [<SPEC>]'],
  ['--bin', '--bin [<NAME>]'],
  ['--test', '--test [<NAME>]'],
  ['--example', '--example [<NAME>]'],
  ['--target', '--target [<TRIPLE>]'],
  ['--profile', '--profile <PROFILE-NAME>'],
  ['-j', '--jobs <N>'],
  ['--jobs', '--jobs <N>'],
]);

const jitter = (ms: number) => Math.round(ms * (0.85 + Math.random() * 0.3));

/** What a cargo command does, here. */
export function script(command: Command): Script {
  const { name, args } = command;
  const last = args[args.length - 1];
  if (last !== undefined && TAKES_VALUE.has(last)) {
    return {
      duration: 140,
      code: 1,
      lines: [
        `${ERR}error${R}: a value is required for '${YELLOW}${TAKES_VALUE.get(last)}${R}' but none was supplied`,
        '',
        `For more information, try '${BOLD}--help${R}'.`,
      ],
    };
  }

  const release = args.includes('-r') || args.includes('--release');
  switch (name) {
    case 'fmt':
      return { duration: jitter(320), code: 0, lines: [] };
    case 'check':
    case 'clippy': {
      const d = jitter(name === 'check' ? 900 : 1300);
      return { duration: d, code: 0, lines: [verb('Checking', `cargo-q v0.1.0 (${ROOT})`), finished('dev', d)] };
    }
    case 'build': {
      const d = jitter(release ? 2100 : 1500);
      return { duration: d, code: 0, lines: [compiling, finished(release ? 'release' : 'dev', d)] };
    }
    case 'doc': {
      const d = jitter(1200);
      return {
        duration: d,
        code: 0,
        lines: [verb('Documenting', 'cargo-q v0.1.0'), finished('dev', d), verb('Generated', `${ROOT}/target/doc/hello_q/index.html`)],
      };
    }
    case 'run': {
      const d = jitter(1000);
      return {
        duration: d,
        code: 0,
        lines: [compiling, finished('dev', d), verb('Running', '`target/debug/cargo-q`'), 'Hello, world!'],
      };
    }
    case 'clean':
      return { duration: jitter(260), code: 0, lines: [verb('Removed', '312 files, 48.2MiB total')] };
    case 'tree':
      return { duration: jitter(300), code: 0, lines: [`cargo-q v0.1.0 (${ROOT})`, '└── lexopt v0.3.1'] };
    case 'update':
      return {
        duration: jitter(700),
        code: 0,
        lines: [verb('Updating', 'crates.io index'), verb('Locking', '0 packages to latest compatible versions')],
      };
    case 'bench': {
      const d = jitter(2400);
      return {
        duration: d,
        code: 0,
        lines: [compiling, finished('bench', d), verb('Running', 'benches/parse.rs'), 'test bench_split ... bench:       1,204 ns/iter (+/- 31)'],
      };
    }
    case 'watch':
      // It never ends on its own: Ctrl-C is the way out.
      return { duration: Number.POSITIVE_INFINITY, code: 0, lines: [`${BOLD}[Running 'cargo check']${R}`, verb('Checking', `cargo-q v0.1.0 (${ROOT})`), `${BOLD}[Finished running. Exit status: 0]${R}`] };
    case 'test':
      return args.includes('--no-run') ? testBuild() : testFailure();
    case 'nextest':
      return {
        duration: jitter(1500),
        code: 100,
        lines: [
          compiling,
          finished('test', 1100),
          `${OK}    Starting${R} 3 tests across 1 binary`,
          `${OK}        PASS${R} [   0.002s] cargo-q tests::adds`,
          `${OK}        PASS${R} [   0.002s] cargo-q tests::parses`,
          `${ERR}        FAIL${R} [   0.003s] cargo-q tests::multiplies`,
          `${ERR}error${R}: test run failed`,
        ],
      };
    default:
      return { duration: jitter(600), code: 0, lines: [] };
  }
}

function testBuild(): Script {
  const d = jitter(1400);
  return {
    duration: d,
    code: 0,
    lines: [compiling, finished('test', d), verb('Executable', 'unittests src/lib.rs (target/debug/deps/hello_q-3f2a1c9e)')],
  };
}

function testFailure(): Script {
  const d = jitter(1700);
  return {
    duration: d,
    code: 101,
    lines: [
      compiling,
      finished('test', d - 300),
      verb('Running', 'unittests src/lib.rs (target/debug/deps/hello_q-3f2a1c9e)'),
      '',
      'running 3 tests',
      `test tests::adds ... ${GREEN}ok${R}`,
      `test tests::parses ... ${GREEN}ok${R}`,
      `test tests::multiplies ... ${RED}FAILED${R}`,
      '',
      'failures:',
      '',
      '---- tests::multiplies stdout ----',
      "thread 'tests::multiplies' panicked at src/lib.rs:21:9:",
      'assertion `left == right` failed',
      '  left: 6',
      ' right: 7',
      '',
      `test result: ${RED}FAILED${R}. 2 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out`,
      '',
      `${ERR}error${R}: test failed, to rerun pass \`--lib\``,
    ],
  };
}

const stripAnsi = (text: string) => text.replace(/\x1b\[[0-9;]*m/g, '');

export const display = (command: Command) =>
  ['cargo', command.name, ...command.args].join(' ');

/** The progress bar: done, running, and what is left (`progress_bar`). */
function progressBar(done: number, running: number, total: number, width: number): string {
  let bar = '';
  let sum = 0;
  for (const [count, ch] of [
    [done, '='],
    [running, '-'],
    [Math.max(0, total - done - running), ' '],
  ] as const) {
    sum += count;
    let size = Math.floor((sum * width) / total);
    if (count > 0 && size === bar.length && size < width) size += 1;
    while (bar.length < size) bar += ch;
  }
  return bar;
}

interface Job {
  command: Command;
  index: number;
  round: number;
}

interface Live extends Job {
  script: Script;
  start: number;
  shown: number;
}

/** Run a list of commands the way cargo-q would. Resolves to the exit code. */
export async function runCargoQ(commands: Command[], options: RunOptions, host: Host): Promise<number> {
  let verbose = options.verbose;
  if (verbose && options.parallel) {
    host.write(
      "warning: -v/--verbose is not supported with -p/--parallel; running quietly\nhelp: use -v/--verbose only to watch each command's output\n",
    );
    verbose = false;
  }
  const fancy = options.fancy && !verbose;
  const color = options.fancy;
  const paint = (text: string) => (color ? text : stripAnsi(text));

  const rounds = options.parallel ? 1 : Math.max(1, options.repeat);
  const queue: Job[] = [];
  for (let round = 0; round < rounds; round += 1) {
    commands.forEach((command, index) => queue.push({ command, index, round }));
  }
  const total = queue.length;
  const slots = options.parallel ? Math.min(commands.length, 8) : 1;
  const running: Live[] = [];

  let clock = 0;
  let succeeded = 0;
  let failed = 0;
  let started = 0;
  let firstCode: number | null = null;
  let pending = '';
  let onScreen = false;

  const frame = () => {
    const cols = host.cols();
    const width = Math.max(10, Math.min(40, cols - 30));
    const done = succeeded + failed;
    let text = onScreen ? '\r\x1b[J' : '';
    text += pending;
    pending = '';
    text += `[${progressBar(done, running.length, total, width)}] ${done}/${total} done, `;
    if (failed > 0) text += `${failed} failed, `;
    text += `${running.length} running\n`;
    let lines = 1;
    for (const task of running.slice(0, 8)) {
      const seconds = Math.floor((clock - task.start) / 1000);
      const note = seconds > 2 ? ` (${seconds}s)` : '';
      let message = display(task.command);
      if (message.length + note.length >= cols) message = `${message.slice(0, Math.max(0, cols - note.length - 4))}...`;
      text += `${message}${note}\n`;
      lines += 1;
    }
    if (running.length > 8) {
      text += `...and ${running.length - 8} more\n`;
      lines += 1;
    }
    host.write(`${text}\x1b[${lines}A`);
    onScreen = true;
  };

  const out = (text: string) => {
    if (fancy) pending += text;
    else host.write(text);
  };

  for (;;) {
    while (running.length < slots && queue.length > 0 && !host.stopped() && (options.keepGoing || failed === 0)) {
      const job = queue.shift()!;
      started += 1;
      running.push({ ...job, script: script(job.command), start: clock, shown: 0 });
      host.task(job.index, 'running', job.round);
      if (!fancy) host.write(`${paint(`${BOLD}[${started}/${total}]${R}`)} ${display(job.command)}\n`);
    }
    if (running.length === 0 || host.stopped()) break;

    if (fancy) frame();
    await host.sleep(TICK);
    if (host.stopped()) break;
    clock += TICK;

    for (const task of [...running]) {
      const { duration, lines, code } = task.script;
      if (verbose) {
        // cargo owns the terminal: its output shows as it comes.
        const due = Number.isFinite(duration)
          ? Math.floor(((clock - task.start) / duration) * (lines.length + 1))
          : Math.floor((clock - task.start) / 400);
        while (task.shown < Math.min(due, lines.length)) host.write(`${lines[task.shown++]}\n`);
      }
      if (clock - task.start < duration) continue;

      running.splice(running.indexOf(task), 1);
      if (code === 0) {
        succeeded += 1;
        host.task(task.index, 'ok', task.round);
        continue;
      }
      failed += 1;
      firstCode ??= code;
      host.task(task.index, 'fail', task.round);
      if (verbose) {
        out(`failed: ${display(task.command)}\n`);
      } else {
        out(`failed: ${display(task.command)} (exit ${code})\n`);
        if (lines.length > 0) out(`${lines.map(paint).join('\n')}\n`);
      }
    }
  }

  const interrupted = host.stopped();
  for (const task of running) host.task(task.index, 'fail', task.round);
  for (const job of queue) host.task(job.index, 'skipped', job.round);

  let tail = fancy && onScreen ? '\r\x1b[J' : '';
  tail += pending;
  if (succeeded !== total) {
    tail += `\n${succeeded} succeeded, ${started - succeeded} failed, ${total - started} skipped\n`;
  }
  const status = interrupted
    ? paint(`${YELLOW}Interrupted${R}`)
    : succeeded === total
      ? paint(`${GREEN}Finished${R}`)
      : paint(`${RED}Failed${R}`);
  tail += `${status} ${total} command(s) in ${(clock / 1000).toFixed(2)}s\n`;
  host.write(tail);

  if (interrupted) return 130;
  return firstCode ?? 0;
}

/** Run one plain cargo command: its own output, as it comes. */
export async function runCargo(command: Command, host: Host): Promise<number> {
  const { duration, lines, code } = script(command);
  let clock = 0;
  let shown = 0;
  while (clock < duration) {
    await host.sleep(TICK);
    if (host.stopped()) return 130;
    clock += TICK;
    const due = Number.isFinite(duration)
      ? Math.floor((clock / duration) * (lines.length + 1))
      : Math.floor(clock / 400);
    while (shown < Math.min(due, lines.length)) host.write(`${lines[shown++]}\n`);
  }
  while (shown < lines.length) host.write(`${lines[shown++]}\n`);
  return code;
}
