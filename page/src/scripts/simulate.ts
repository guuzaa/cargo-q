/**
 * What a run of cargo-q would look like.
 *
 * The cargo commands are simulated: the page shows the progress display,
 * the failures and the summary of a run, without running anything. The
 * commands themselves come from cargo-q's own parser, so what is simulated
 * is the running, not the splitting.
 */

/** One command of a run. */
export interface Command {
  name: string;
  args: string[];
}

/** The options of a run that change how it goes. */
export interface Options {
  /** Run the commands at once instead of one after another. */
  parallel: boolean;
  /** Keep going after a failure. */
  keepGoing: boolean;
  /** Print the commands and run nothing. */
  dryRun: boolean;
  /** Show the overprinting status display, rather than one line per command. */
  fancy: boolean;
  /** How many times the whole list runs. */
  repeat: number;
}

/** What to write to the terminal, and how long to wait before the next. */
export interface Frame {
  text: string;
  wait: number;
}

const RESET = '\x1b[0m';
const GREEN = '\x1b[32m';
const RED = '\x1b[31m';
const YELLOW = '\x1b[33m';

/** How wide the progress bar is. */
const BAR_WIDTH = 40;

/** How often the progress display is redrawn. */
const TICK = 80;

/** How long each command takes, in milliseconds. */
const DURATIONS: Record<string, number> = {
  fmt: 250,
  clean: 200,
  tree: 300,
  check: 700,
  clippy: 900,
  doc: 600,
  test: 800,
  build: 1100,
  run: 700,
};

/** The commands that fail, and what they print when they do. */
const FAILURES: Record<string, { code: number; output: string }> = {
  test: {
    code: 101,
    output: [
      '     Running unittests src/lib.rs (target/debug/deps/demo-8a3f1c)',
      'running 2 tests',
      'test adds ... ok',
      'test multiplies ... FAILED',
      '',
      'failures:',
      '',
      '---- multiplies stdout ----',
      '',
      "thread 'multiplies' panicked at tests/math.rs:10:5:",
      'assertion `left == right` failed',
      '  left: 6',
      ' right: 7',
      '',
      `${RED}error${RESET}: test failed, to rerun pass \`--test math\``,
    ].join('\n'),
  },
};

/** How long a command takes. */
function durationOf(command: Command): number {
  return DURATIONS[command.name] ?? 500;
}

/** Whether a command fails, and what it says when it does. */
function failureOf(command: Command): { code: number; output: string } | null {
  return FAILURES[command.name] ?? null;
}

/** The progress bar: done, running, and what is left. */
function progressBar(completed: number, running: number, total: number): string {
  let bar = '';
  let sum = 0;
  const pending = Math.max(0, total - completed - running);

  for (const [count, character] of [
    [completed, '='],
    [running, '-'],
    [pending, ' '],
  ] as const) {
    sum += count;
    let size = Math.floor((sum * BAR_WIDTH) / total);
    if (count > 0 && size === bar.length && size < BAR_WIDTH) size += 1;
    while (bar.length < size) bar += character;
  }

  return bar;
}

/** The command, the way cargo-q writes it. */
function display(command: Command): string {
  const args = command.args.length > 0 ? ` ${command.args.join(' ')}` : '';
  return `cargo ${command.name}${args}`;
}

/** A run of a list of commands, simulated. */
export class Simulation {
  private readonly commands: Command[];
  private readonly options: Options;
  private stopped = false;

  constructor(commands: Command[], options: Options) {
    this.commands = commands;
    this.options = options;
  }

  /** Ask the run to stop, the way Ctrl-C does. */
  stop(): void {
    this.stopped = true;
  }

  /**
   * Run the commands, and say what happened.
   *
   * The frames are handed to `paint` one at a time, each with the time to
   * wait before the next one. The return value is the exit code the run
   * would leave behind.
   */
  async run(paint: (frame: Frame) => Promise<void>): Promise<number> {
    const rounds = Math.max(1, this.options.repeat);
    const total = this.commands.length * rounds;
    let succeeded = 0;
    let failed = 0;
    let started = 0;
    let exitCode = 0;
    let interrupted = false;
    let elapsed = 0;
    let first = true;

    /** The failure lines, printed after the progress display is cleared. */
    const failures: string[] = [];

    /** The progress display, as cargo-q writes it. */
    const frame = (completed: number, running: Command[]) => {
      const bar = progressBar(completed, running.length, total);
      const failedPart = failed > 0 ? `${failed} failed, ` : '';
      let text = `[${bar}] ${completed + failed}/${total} done, ${failedPart}${running.length} running`;

      for (const command of running) {
        const seconds = Math.floor(elapsed / 1000);
        const note = seconds > 2 ? ` (${seconds}s)` : '';
        text += `\r\n${display(command)}${note}`;
      }

      // Every frame after the first clears the one before it.
      const prefix = first ? '' : '\r\x1b[J';
      first = false;

      const lines = running.length + 1;
      return `${prefix}${text}\r\n\x1b[${lines}A`;
    };

    /** Clear the progress display, so that what comes after stays. */
    const clear = () => '\r\x1b[J';

    /** One line per command, the way a plain console writes it. */
    const plain = (command: Command, index: number) => `[${index}/${total}] ${display(command)}\r\n`;

    /** Run one command, and say whether it failed. */
    const runCommand = async (command: Command): Promise<boolean> => {
      started += 1;
      const failure = failureOf(command);
      const until = elapsed + durationOf(command);

      while (elapsed < until && !this.stopped) {
        if (this.options.fancy) {
          const step = Math.min(TICK, until - elapsed);
          await paint({ text: frame(succeeded + failed, [command]), wait: step });
          elapsed += step;
        } else {
          elapsed = until;
        }
      }

      if (this.stopped) return false;

      if (failure) {
        failed += 1;
        if (exitCode === 0) exitCode = failure.code;
        failures.push(this.failureText(command, failure));
        return true;
      }

      succeeded += 1;
      return false;
    };

    if (this.options.parallel) {
      // Everything starts at once; a failure does not stop what is running.
      const running = new Set(this.commands);
      const finishes = this.commands.map((command) => elapsed + durationOf(command));

      while (running.size > 0 && !this.stopped) {
        await paint({ text: frame(succeeded + failed, [...running]), wait: TICK });
        elapsed += TICK;

        for (const command of [...running]) {
          const index = this.commands.indexOf(command);
          if (elapsed < finishes[index]) continue;
          running.delete(command);
          started += 1;

          const failure = failureOf(command);
          if (failure) {
            failed += 1;
            if (exitCode === 0) exitCode = failure.code;
            failures.push(this.failureText(command, failure));
          } else {
            succeeded += 1;
          }
        }
      }
    } else {
      let index = 0;

      for (let round = 0; round < rounds; round += 1) {
        for (const command of this.commands) {
          if (this.stopped) break;
          index += 1;

          if (!this.options.fancy) {
            await paint({ text: plain(command, index), wait: 0 });
          }

          const failedNow = await runCommand(command);
          if (failedNow && !this.options.keepGoing) {
            // The run ends here: the later rounds never start.
            round = rounds;
            break;
          }        }
      }
    }

    if (this.stopped) {
      interrupted = true;
      exitCode = 130;
    }

    // What the run leaves behind: the failures, then the summary.
    let tail = this.options.fancy ? clear() : '';
    for (const failure of failures) tail += failure;

    const seconds = (elapsed / 1000).toFixed(2);
    if (interrupted) {
      tail += `\r\n${YELLOW}Interrupted${RESET} ${total} command(s) in ${seconds}s\r\n`;
    } else if (failed === 0 && succeeded === total) {
      tail += `\r\n${GREEN}Finished${RESET} ${total} command(s) in ${seconds}s\r\n`;
    } else {
      const skipped = Math.max(0, total - started);
      tail += `\r\n${succeeded} succeeded, ${failed} failed, ${skipped} skipped\r\n`;
      tail += `${RED}Failed${RESET} ${total} command(s) in ${seconds}s\r\n`;
    }

    await paint({ text: tail, wait: 0 });
    return exitCode;
  }

  /** The line a failed command leaves behind, and its output. */
  private failureText(command: Command, failure: { code: number; output: string }): string {
    const output = failure.output.replace(/\n/g, '\r\n');
    return `\r\n${RED}failed${RESET}: ${display(command)} (exit ${failure.code})\r\n${output}\r\n`;
  }
}
