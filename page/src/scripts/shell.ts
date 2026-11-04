/**
 * A shell for cargo-q, in a terminal.
 *
 * What you type is parsed by cargo-q's own parser, running in the browser,
 * and the cargo commands are simulated. So the split is the real one, and
 * the running is a rehearsal of it.
 */
import { inspect, version, wellKnown } from './wasm';
import { cargoQArgs, checkCargoQ, tokenize } from './tokens';
import { Simulation, type Command, type Options } from './simulate';

const RESET = '\x1b[0m';
const RUST = '\x1b[38;2;242;107;33m';
const SAND = '\x1b[38;2;240;233;223m';
const DIM = '\x1b[38;2;139;129;115m';
const RED = '\x1b[38;2;255;123;114m';

const PROMPT = `${RUST}$ ${RESET}`;

/** The third-party subcommands people usually have. */
export const EXTRA_NAMES = [
  'nextest',
  'watch',
  'deny',
  'audit',
  'llvm-cov',
  'tarpaulin',
  'expand',
  'udeps',
  'bloat',
  'install',
];

/** The part of a terminal the shell needs. */
export interface Writer {
  write(data: string): void;
  writeln(data: string): void;
  clear(): void;
}

/** A shell for cargo-q. */
export class Shell {
  private readonly terminal: Writer;
  private readonly names: string[];
  private buffer = '';
  private cursor = 0;
  private readonly history: string[] = [];
  private historyAt = -1;
  private status = 0;
  private running: Simulation | null = null;
  private version = '';

  constructor(terminal: Writer, names: string[]) {
    this.terminal = terminal;
    this.names = names;
  }

  /** Say hello, and wait for a command line. */
  start(): void {
    const name = this.version ? `cargo-q ${this.version}` : 'cargo-q';
    this.terminal.writeln(
      `${DIM}${name} — the cargo commands are simulated here, and \`cargo test\` fails on purpose.${RESET}`,
    );
    this.terminal.writeln(
      `${DIM}Type a command line that starts with "cargo q" and press Enter. Try: cargo q fmt clippy check test${RESET}`,
    );
    this.terminal.write('\r\n');
    this.prompt();
  }

  /** The version of cargo-q, for the greeting. */
  async setVersion(): Promise<void> {
    this.version = await version();
  }

  /** The names of the cargo subcommands the shell knows. */
  async setNames(): Promise<void> {
    this.names.push(...EXTRA_NAMES, ...(await wellKnown()));
  }

  /** Write a line into the terminal, and run it. */
  async typeAndRun(line: string): Promise<void> {
    if (this.running) return;
    this.terminal.write(line);
    await this.submit();
  }

  /** What the keyboard does. */
  onData(data: string): void {
    // While a run is going, only Ctrl-C gets through.
    if (this.running) {
      if (data === '\x03') this.running.stop();
      return;
    }

    switch (data) {
      case '\r':
        void this.submit();
        return;
      case '\x7f':
        this.backspace();
        return;
      case '\x15':
        this.buffer = '';
        this.cursor = 0;
        this.redraw();
        return;
      case '\x0c':
        this.terminal.clear();
        this.prompt();
        return;
      case '\x03':
        this.terminal.write('^C\r\n');
        this.prompt();
        return;
      case '\x1b[A':
        this.historyMove(1);
        return;
      case '\x1b[B':
        this.historyMove(-1);
        return;
      case '\x1b[C':
        this.move(1);
        return;
      case '\x1b[D':
        this.move(-1);
        return;
      default:
        break;
    }

    const text = data.replace(/[\x00-\x1f\x7f]/g, '');
    if (!text) return;
    this.insert(text);
  }

  /** Put the text into the line, where the cursor is. */
  private insert(text: string): void {
    this.buffer = this.buffer.slice(0, this.cursor) + text + this.buffer.slice(this.cursor);
    this.cursor += text.length;
    this.redraw();
  }

  /** Take the character before the cursor away. */
  private backspace(): void {
    if (this.cursor === 0) return;
    this.buffer = this.buffer.slice(0, this.cursor - 1) + this.buffer.slice(this.cursor);
    this.cursor -= 1;
    this.redraw();
  }

  /** Move the cursor along the line. */
  private move(step: number): void {
    this.cursor = Math.max(0, Math.min(this.buffer.length, this.cursor + step));
    this.redraw();
  }

  /** Walk through the lines that were run before. */
  private historyMove(direction: number): void {
    const next = this.historyAt + direction;
    if (next < -1 || next >= this.history.length) return;
    this.historyAt = next;
    this.buffer = next === -1 ? '' : this.history[next];
    this.cursor = this.buffer.length;
    this.redraw();
  }

  /** Write the line again, with the cursor where it belongs. */
  private redraw(): void {
    this.terminal.write(`\r${PROMPT}${this.buffer}\x1b[K`);
    const back = this.buffer.length - this.cursor;
    if (back > 0) this.terminal.write(`\x1b[${back}D`);
  }

  /** The prompt, at the end of a line. */
  private prompt(): void {
    this.terminal.write(PROMPT);
  }

  /** Run the line, and answer it. */
  private async submit(): Promise<void> {
    const line = this.buffer;
    this.terminal.write('\r\n');
    this.buffer = '';
    this.cursor = 0;

    if (line.trim() !== '') {
      this.history.unshift(line);
      this.historyAt = -1;
    }

    try {
      await this.run(line);
    } catch (error) {
      // The parser could not be loaded, most likely: say so, and keep the
      // shell usable for the next line.
      const message = error instanceof Error ? error.message : String(error);
      this.terminal.writeln(`${RED}${message}${RESET}`);
    } finally {
      this.running = null;
    }
    this.prompt();
  }

  /** What a line of input means, and what it does. */
  private async run(line: string): Promise<void> {
    const trimmed = line.trim();
    if (trimmed === '') return;

    if (trimmed === 'clear') {
      this.terminal.clear();
      return;
    }

    if (/^echo\s+\$\?$/.test(trimmed)) {
      this.terminal.writeln(String(this.status));
      return;
    }

    const { tokens, error } = tokenize(line);
    if (error) {
      this.printError(error);
      return;
    }

    // Only `cargo q` names the subcommand.
    const prefix = checkCargoQ(tokens);
    if (prefix) {
      this.printError(prefix);
      return;
    }

    const args = cargoQArgs(tokens);
    const inspection = await inspect(
      args.map((token) => token.text),
      this.names,
    );

    if (inspection.help) {
      // The terminal does not turn `\n` into a new line by itself
      // (`convertEol` is off, so the progress display can overprint).
      this.terminal.write(inspection.help.replace(/\n/g, '\r\n'));
      return;
    }

    if (inspection.version) {
      this.terminal.writeln(`cargo-q ${inspection.version}`);
      return;
    }

    if (inspection.error) {
      this.printError(inspection.error);
      return;
    }

    const commands: Command[] = inspection.commands.map((command) => ({
      name: command.name,
      args: command.args,
    }));

    const options: Options = {
      parallel: inspection.options.includes('--parallel'),
      keepGoing: inspection.options.includes('--keep-going'),
      dryRun: inspection.options.includes('--dry-run'),
      fancy: !inspection.options.includes('--no-fancy'),
      repeat: Number(
        inspection.options
          .find((option) => option.startsWith('--repeat '))
          ?.slice('--repeat '.length) ?? 1,
      ),
    };

    if (options.dryRun) {
      this.printPlan(commands, options);
      return;
    }

    const simulation = new Simulation(commands, options);
    this.running = simulation;
    this.status = await simulation.run(async (frame) => {
      this.terminal.write(frame.text);
      if (frame.wait > 0) {
        await new Promise((resolve) => setTimeout(resolve, frame.wait));
      }
    });
    this.running = null;
  }

  /** An error, the way cargo-q prints one: the first line in red. */
  private printError(message: string): void {
    const [first, ...rest] = message.split('\n');
    this.terminal.writeln(`${RED}${first}${RESET}`);
    for (const part of rest) this.terminal.writeln(part);
  }

  /** The commands a line would run, the way `--dry-run` prints them. */
  private printPlan(commands: Command[], options: Options): void {
    const rounds = options.repeat > 1 ? ` x ${options.repeat} rounds` : '';
    const mode = options.parallel ? 'in parallel' : 'sequentially';
    this.terminal.writeln(`${SAND}${commands.length} command(s)${rounds}, ${mode}:${RESET}`);

    for (const command of commands) {
      const args = command.args.join(' ');
      this.terminal.writeln(
        `  ${SAND}cargo ${RESET}${RUST}${command.name}${RESET}${args ? ` ${SAND}${args}${RESET}` : ''}`,
      );
    }
  }
}
