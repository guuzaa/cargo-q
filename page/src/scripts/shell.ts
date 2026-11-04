/**
 * A small shell in an xterm.js terminal.
 *
 * The line is colored as it is typed, from what cargo-q's parser (wasm)
 * makes of it. `cargo q …` is parsed by that parser and its commands are
 * run by the simulation; everything else is a handful of builtins.
 */

import type { Terminal } from '@xterm/xterm';
import { analyze, isCargoQ, PROGRAMS, type Analysis, type Role } from './analyze';
import { runCargo, runCargoQ, type Host, type RunOptions, type TaskState } from './simulate';
import { segments, tokenize } from './tokens';
import type { Inspection, Parser } from './wasm';
import { PALETTE, ansi } from './palette';

const R = '\x1b[0m';
const C = {
  rust: ansi(PALETTE.rust[500]),
  sand: ansi(PALETTE.sand[100]),
  dim: ansi(PALETTE.sand[500]),
  ghost: ansi(PALETTE.sand[600]),
  ok: ansi(PALETTE.status.ok),
  fail: ansi(PALETTE.status.fail),
  warn: ansi(PALETTE.status.warn),
  info: ansi(PALETTE.status.info),
  peach: ansi(PALETTE.rust[300]),
};

const ROLE: Record<Role, string> = {
  program: `\x1b[1m${C.sand}`,
  sub: `\x1b[1m${C.rust}`,
  option: C.warn,
  command: `\x1b[1m${C.ok}`,
  arg: C.info,
  error: `\x1b[4m${C.fail}`,
  after: C.dim,
  unknown: C.fail,
  word: C.sand,
  op: C.peach,
};

const PROMPT_TEXT = '~/cargo-q ❯ ';

const CARGOQ_FLAGS = ['--verbose', '--parallel', '--keep-going', '--dry-run', '--no-fancy', '--repeat', '--help', '--version'];
const CARGO_FLAGS = ['--release', '--no-run', '--all-targets', '--workspace', '--check', '--features=', '--lib', '--locked', '--quiet'];

/** What the shell tells the page around it. */
export interface ShellEvents {
  input(analysis: Analysis): void;
  run(inspection: Inspection, options: RunOptions): void;
  task(index: number, state: TaskState, round: number): void;
  exit(code: number, ranCargoQ: boolean): void;
}

const sleep = (ms: number) => new Promise<void>((resolve) => setTimeout(resolve, ms));

export class Shell {
  private parser: Parser | null = null;
  private buffer = '';
  private cursor = 0;
  /** The row of the cursor, counted from the row the prompt is on. */
  private row = 0;
  private status = 0;
  private history: string[] = [];
  private historyAt = -1;
  private draft = '';
  private busy = false;
  private interrupted = false;
  private idle: Promise<void> = Promise.resolve();
  private typing = 0;
  /**
   * Whether the gray suggestion is out of the way.
   *
   * The suggestion is a preview of a line the shell knows, drawn after the
   * cursor. It must never fight the person typing: a deletion drops it, so
   * what is left on the screen is only what was typed. Typing offers it
   * again, and so does the key that accepts it.
   */
  private hintOff = false;

  constructor(
    private readonly term: Terminal,
    private readonly parserReady: Promise<Parser>,
    private readonly events: ShellEvents,
    private readonly suggestions: string[],
  ) {
    void parserReady.then((parser) => {
      this.parser = parser;
      if (!this.busy) this.render();
    });
    term.onData((data) => this.onData(data));
  }

  start(greeting: string): void {
    this.term.write(greeting);
    this.prompt();
  }

  focus(): void {
    this.term.focus();
  }

  /** Type a line as a person would, then run it. */
  async type(line: string, options: { run?: boolean; instant?: boolean } = {}): Promise<void> {
    const ticket = ++this.typing;
    if (this.busy) {
      this.interrupted = true;
      await this.idle;
      await sleep(250);
    }
    if (ticket !== this.typing) return;
    this.setBuffer('');
    for (const ch of options.instant ? [line] : [...line]) {
      if (ticket !== this.typing) return;
      this.insert(ch);
      if (!options.instant) await sleep(ch === ' ' ? 70 + Math.random() * 60 : 26 + Math.random() * 40);
    }
    if (options.run === false || ticket !== this.typing) return;
    await sleep(options.instant ? 0 : 320);
    if (ticket === this.typing && !this.busy) void this.enter();
  }

  // --- input -------------------------------------------------------------

  private onData(data: string): void {
    if (this.busy) {
      if (data.includes('\x03')) {
        this.interrupted = true;
        this.term.write('^C');
      }
      return;
    }
    // A key from the person stops any typing in progress.
    this.typing += 1;

    const keys = data.match(/\x1b\[[0-9;]*[A-Za-z~]|\x1bO[A-Z]|\x1b.|[\s\S]/g) ?? [];
    for (const key of keys) {
      switch (key) {
        case '\r':
        case '\n':
          void this.enter();
          return;
        case '\x7f':
        case '\x08':
          if (this.cursor > 0) {
            this.hintOff = true;
            this.buffer = this.buffer.slice(0, this.cursor - 1) + this.buffer.slice(this.cursor);
            this.cursor -= 1;
          }
          break;
        case '\x1b[3~':
          if (this.cursor < this.buffer.length) {
            this.hintOff = true;
            this.buffer = this.buffer.slice(0, this.cursor) + this.buffer.slice(this.cursor + 1);
          }
          break;
        case '\x1b[D':
        case '\x02':
          this.cursor = Math.max(0, this.cursor - 1);
          break;
        case '\x1b[C':
        case '\x06':
          if (this.cursor === this.buffer.length) {
            this.hintOff = false;
            this.acceptSuggestion();
          } else this.cursor += 1;
          break;
        case '\x1b[1;5D':
        case '\x1bb':
          this.cursor = this.wordStart(this.cursor);
          break;
        case '\x1b[1;5C':
        case '\x1bf':
          this.cursor = this.wordEnd(this.cursor);
          break;
        case '\x1b[H':
        case '\x1bOH':
        case '\x1b[1~':
        case '\x01':
          this.cursor = 0;
          break;
        case '\x1b[F':
        case '\x1bOF':
        case '\x1b[4~':
        case '\x05':
          this.hintOff = false;
          if (this.cursor === this.buffer.length) this.acceptSuggestion();
          this.cursor = this.buffer.length;
          break;
        case '\x1b[A':
        case '\x10':
          this.recall(1);
          break;
        case '\x1b[B':
        case '\x0e':
          this.recall(-1);
          break;
        case '\x15':
          this.hintOff = true;
          this.buffer = this.buffer.slice(this.cursor);
          this.cursor = 0;
          break;
        case '\x0b':
          this.hintOff = true;
          this.buffer = this.buffer.slice(0, this.cursor);
          break;
        case '\x17': {
          this.hintOff = true;
          const start = this.wordStart(this.cursor);
          this.buffer = this.buffer.slice(0, start) + this.buffer.slice(this.cursor);
          this.cursor = start;
          break;
        }
        case '\x03':
          this.term.write(`${this.draw(false)}^C\r\n`);
          this.buffer = '';
          this.cursor = 0;
          this.setStatus(130, false);
          this.prompt();
          return;
        case '\x0c':
          this.term.write('\x1b[H\x1b[2J');
          this.row = 0;
          break;
        case '\x04':
          if (this.buffer === '') {
            void this.type('exit', { instant: true });
            return;
          }
          break;
        case '\t':
          this.complete();
          return;
        default:
          if (key.length === 1 && key >= ' ') {
            this.hintOff = false;
            this.buffer = this.buffer.slice(0, this.cursor) + key + this.buffer.slice(this.cursor);
            this.cursor += 1;
          }
      }
    }
    this.render();
  }

  private insert(text: string): void {
    this.hintOff = false;
    this.buffer = this.buffer.slice(0, this.cursor) + text + this.buffer.slice(this.cursor);
    this.cursor += text.length;
    this.render();
  }

  private setBuffer(text: string): void {
    this.hintOff = false;
    this.buffer = text;
    this.cursor = text.length;
    this.render();
  }

  private wordStart(at: number): number {
    let i = at;
    while (i > 0 && this.buffer[i - 1] === ' ') i -= 1;
    while (i > 0 && this.buffer[i - 1] !== ' ') i -= 1;
    return i;
  }

  private wordEnd(at: number): number {
    let i = at;
    while (i < this.buffer.length && this.buffer[i] === ' ') i += 1;
    while (i < this.buffer.length && this.buffer[i] !== ' ') i += 1;
    return i;
  }

  private recall(step: number): void {
    if (this.history.length === 0) return;
    if (this.historyAt === -1) this.draft = this.buffer;
    const at = Math.min(this.history.length - 1, Math.max(-1, this.historyAt + step));
    this.historyAt = at;
    const line = at === -1 ? this.draft : this.history[this.history.length - 1 - at];
    // A line from the history is whole: there is nothing left to suggest.
    this.hintOff = true;
    this.buffer = line;
    this.cursor = line.length;
  }

  /** The rest of a line the shell has seen before, offered in gray. */
  private suggestion(): string {
    if (this.hintOff) return '';
    if (this.buffer.trim() === '' || this.cursor !== this.buffer.length) return '';
    const pool = [...[...this.history].reverse(), ...this.suggestions];
    const hit = pool.find((line) => line.startsWith(this.buffer) && line !== this.buffer);
    return hit ? hit.slice(this.buffer.length) : '';
  }

  private acceptSuggestion(): void {
    const rest = this.suggestion();
    if (rest) {
      this.buffer += rest;
      this.cursor = this.buffer.length;
    }
  }

  private complete(): void {
    const before = this.buffer.slice(0, this.cursor);
    const start = before.lastIndexOf(' ') + 1;
    const word = before.slice(start);
    const { tokens } = tokenize(before.slice(0, start));
    const words = (segments(tokens).pop()?.words ?? []).map((token) => token.text);

    let pool: string[];
    if (words.length === 0) pool = PROGRAMS;
    else if (words[0] !== 'cargo') pool = words[0] === 'cat' ? ['Cargo.toml'] : [];
    else if (words.length === 1) pool = ['q', ...(this.parser?.names ?? [])];
    else if (word.startsWith('-')) {
      const inOptions = words[1] === 'q' && !(this.parser?.inspect(words).commands.length ?? 0);
      pool = inOptions ? CARGOQ_FLAGS : CARGO_FLAGS;
    } else pool = this.parser?.names ?? [];

    const matches = [...new Set(pool)].filter((name) => name.startsWith(word));
    if (matches.length === 0) {
      this.render();
      this.term.write('\x07');
      return;
    }
    // The tab answered the question, so the gray suggestion has nothing to
    // say: it would only hide what was completed.
    this.hintOff = true;
    let common = matches[0];
    for (const match of matches) while (!match.startsWith(common)) common = common.slice(0, -1);
    if (matches.length === 1) {
      common += common.endsWith('=') ? '' : ' ';
    } else if (common === word) {
      // Nothing more to add: show what there is to choose from.
      this.menu(matches);
      return;
    }
    this.buffer = before.slice(0, start) + common + this.buffer.slice(this.cursor);
    this.cursor = start + common.length;
    this.render();
  }

  /**
   * The candidates, drawn under the prompt.
   *
   * The menu is a choice, not a line the shell has run: it is gone with the
   * next keystroke, which erases everything under the prompt. So it is
   * drawn there in the first place, and the cursor is put back where it
   * belongs right after.
   */
  private menu(matches: string[]): void {
    const cols = this.term.cols;
    const width = Math.max(...matches.map((m) => m.length)) + 2;
    const perRow = Math.max(1, Math.floor(cols / width));
    // The menu has to fit on the screen: a longer one would push the
    // prompt off the top of it, and then it could not be taken away.
    const room = Math.max(2, (this.term.rows - 2) * perRow);
    const shown = matches.length > room ? room - 1 : matches.length;

    let list = '';
    for (let i = 0; i < shown; i += 1) {
      list += `${C.dim}${matches[i].padEnd(width)}${R}`;
      if ((i + 1) % perRow === 0 && i + 1 < shown) list += '\r\n';
    }
    if (shown < matches.length) list += `${C.dim}…+${matches.length - shown}${R}`;
    const rows = Math.ceil(shown / perRow);

    // The prompt is drawn again, so the menu starts under it and not under
    // whatever was on the screen before.
    const prompt = this.draw(false);
    const from = this.row;
    this.term.write(`${prompt}\r\n${list}`);

    // The cursor is on the last row of the menu now; it goes back to the
    // line, so the next keystroke can erase the menu with it.
    const live = PROMPT_TEXT.length + this.cursor;
    const liveRow = Math.floor(live / cols);
    this.term.write(`\x1b[${from + rows - liveRow}A\x1b[${(live % cols) + 1}G`);
    this.row = liveRow;
  }

  // --- drawing -----------------------------------------------------------

  private prompt(): void {
    this.row = 0;
    this.historyAt = -1;
    this.hintOff = false;
    // The split view keeps showing the line that just ran until the next
    // key, so the new prompt does not tell it about the empty line.
    this.term.write(this.draw(true, true, false));
  }

  private promptText(): string {
    const mark = this.status === 0 ? C.ok : C.fail;
    return `${C.rust}~/cargo-q${R} ${mark}❯${R} `;
  }

  /**
   * The escape sequence that redraws the prompt and the line, leaving the
   * cursor where it belongs (or at the end, without the gray suggestion).
   */
  private draw(live: boolean, fresh = false, emit = live): string {
    const cols = this.term.cols;
    const analysis = analyze(this.buffer, this.cursor, this.parser);
    if (emit) this.events.input(analysis);

    let line = '';
    let at = 0;
    for (const token of analysis.tokens) {
      line += this.buffer.slice(at, token.start);
      line += `${ROLE[token.role]}${this.buffer.slice(token.start, token.end)}${R}`;
      at = token.end;
    }
    line += this.buffer.slice(at);
    const ghost = live ? this.suggestion() : '';

    let out = fresh ? '' : `${this.row > 0 ? `\x1b[${this.row}A` : ''}\r\x1b[J`;
    out += this.promptText() + line + (ghost ? `${C.ghost}${ghost}${R}` : '');

    const total = PROMPT_TEXT.length + this.buffer.length + ghost.length;
    // Past the last column the cursor waits to wrap; make it wrap now.
    if (total > 0 && total % cols === 0) out += '\r\n';
    const endRow = Math.floor(total / cols);
    const target = PROMPT_TEXT.length + (live ? this.cursor : this.buffer.length);
    const row = Math.floor(target / cols);
    if (endRow > row) out += `\x1b[${endRow - row}A`;
    out += `\x1b[${(target % cols) + 1}G`;
    this.row = row;
    return out;
  }

  private render(fresh = false): void {
    this.term.write(this.draw(true, fresh));
  }

  private setStatus(code: number, ranCargoQ: boolean): void {
    this.status = code;
    this.events.exit(code, ranCargoQ);
  }

  // --- running -----------------------------------------------------------

  private async enter(): Promise<void> {
    const line = this.buffer;
    this.term.write(`${this.draw(false)}\r\n`);
    this.buffer = '';
    this.cursor = 0;
    if (line.trim() !== '') {
      if (this.history[this.history.length - 1] !== line) this.history.push(line);
      let release!: () => void;
      this.idle = new Promise((resolve) => (release = resolve));
      this.busy = true;
      this.interrupted = false;
      try {
        await this.execute(line);
      } finally {
        this.busy = false;
        release();
      }
    }
    this.prompt();
  }

  private async execute(line: string): Promise<void> {
    const { tokens, open } = tokenize(line);
    if (open) {
      this.term.write(`sh: unexpected EOF while looking for matching quote\r\n`);
      this.setStatus(2, false);
      return;
    }
    let previous: string | null = null;
    for (const { words, op } of segments(tokens)) {
      const skip = (previous === '&&' && this.status !== 0) || (previous === '||' && this.status === 0);
      previous = op;
      if (skip || words.length === 0) continue;
      const texts = words.map((word) => word.text.replace(/\$\?/g, String(this.status)));
      const { code, cargoQ } = await this.run(texts, isCargoQ(words));
      this.setStatus(code, cargoQ);
      if (this.interrupted) break;
    }
  }

  private host(): Host {
    return {
      write: (text) => this.term.write(text.replace(/\n/g, '\r\n')),
      sleep,
      stopped: () => this.interrupted,
      cols: () => this.term.cols,
      task: (index, state, round) => this.events.task(index, state, round),
    };
  }

  private out(text: string): void {
    this.term.write(`${text.replace(/\n/g, '\r\n')}\r\n`);
  }

  private async run(words: string[], cargoQ: boolean): Promise<{ code: number; cargoQ: boolean }> {
    const [program, ...args] = words;
    switch (program) {
      case 'cargo': {
        if (cargoQ) return this.cargoQ(words);
        const code = await this.cargo(args);
        if (this.interrupted) this.term.write('\r\n');
        return { code, cargoQ: false };
      }
      case 'echo':
        this.out(args.join(' '));
        return { code: 0, cargoQ: false };
      case 'clear':
        this.term.write('\x1b[H\x1b[2J\x1b[3J');
        return { code: 0, cargoQ: false };
      case 'history':
        this.out(this.history.map((h, i) => `${C.dim}${String(i + 1).padStart(4)}${R}  ${h}`).join('\n'));
        return { code: 0, cargoQ: false };
      case 'pwd':
        this.out('/home/you/cargo-q');
        return { code: 0, cargoQ: false };
      case 'ls':
        this.out(`${C.sand}Cargo.lock  Cargo.toml${R}  ${C.info}\x1b[1msrc  target  tests${R}`);
        return { code: 0, cargoQ: false };
      case 'cat':
        if (args[0] === 'Cargo.toml') {
          this.out(`${C.peach}[package]${R}\nname = ${C.ok}"cargo-q"${R}\nversion = ${C.ok}"0.1.0"${R}\nedition = ${C.ok}"2021"${R}\n\n${C.peach}[dependencies]${R}\nlexopt = ${C.ok}"0.3"${R}`);
          return { code: 0, cargoQ: false };
        }
        this.out(`cat: ${args[0] ?? ''}: No such file or directory`);
        return { code: 1, cargoQ: false };
      case 'true':
        return { code: 0, cargoQ: false };
      case 'false':
        return { code: 1, cargoQ: false };
      case 'help':
        this.out(
          [
            `${C.dim}commands${R}  cargo q …  cargo …  echo $?  ls  cat  history  clear`,
            `${C.dim}keys${R}      ↑ ↓ history   tab complete   → accept   ^C stop   ^L clear`,
            `${C.dim}chain${R}     &&  ||  ;`,
          ].join('\n'),
        );
        return { code: 0, cargoQ: false };
      case 'exit':
        this.out(`${C.dim}logout${R}`);
        await sleep(500);
        this.term.write('\x1b[H\x1b[2J\x1b[3J');
        this.out(`${C.dim}… and back. There is no leaving a landing page.${R}\n`);
        return { code: 0, cargoQ: false };
      default:
        this.out(`sh: ${program}: command not found`);
        return { code: 127, cargoQ: false };
    }
  }

  private async cargo(args: string[]): Promise<number> {
    const [sub, ...rest] = args;
    if (sub === undefined || sub === '-h' || sub === '--help') {
      this.out(`Rust's package manager\n\n${C.dim}Usage:${R} cargo [+toolchain] [OPTIONS] [COMMAND]\n\n${C.dim}Running several? Try${R} cargo q check test`);
      return 0;
    }
    if (sub === '-V' || sub === '--version') {
      this.out('cargo 1.91.0 (ea2d97820 2025-10-10)');
      return 0;
    }
    if (sub === '--list') {
      this.out(`Installed Commands:\n${(this.parser?.names ?? []).map((n) => `    ${n}`).join('\n')}\n    q`);
      return 0;
    }
    const parser = this.parser ?? (await this.parserReady);
    if (!parser.names.includes(sub)) {
      this.out(`\x1b[1;31merror\x1b[0m: no such command: \`${sub}\`\n\n\x1b[1;36mhelp\x1b[0m: view all installed commands with \`cargo --list\``);
      return 101;
    }
    return runCargo({ name: sub, args: rest }, this.host());
  }

  private async cargoQ(argv: string[]): Promise<{ code: number; cargoQ: boolean }> {
    if (!this.parser) this.out(`${C.dim}loading the parser…${R}`);
    const parser = this.parser ?? (await this.parserReady);
    const inspection = parser.inspect(argv);

    if (inspection.help) {
      this.out(inspection.help.trimEnd());
      return { code: inspection.exitCode ?? 0, cargoQ: false };
    }
    if (inspection.version) {
      this.out(`cargo-q ${inspection.version}`);
      return { code: inspection.exitCode ?? 0, cargoQ: false };
    }
    if (inspection.error) {
      const error = inspection.error.replace(/^error:/, '\x1b[1;31merror\x1b[0m:').replace(/\nhelp:/g, '\n\x1b[1;36mhelp\x1b[0m:');
      this.out(error.trimEnd());
      return { code: inspection.exitCode ?? 1, cargoQ: true };
    }

    const { settings } = inspection;
    if (settings.dryRun && inspection.plan) {
      this.out(inspection.plan.trimEnd());
      return { code: 0, cargoQ: false };
    }

    const options: RunOptions = {
      parallel: settings.parallel,
      keepGoing: settings.keepGoing,
      fancy: !settings.noFancy,
      verbose: settings.verbose,
      repeat: settings.repeat,
    };
    this.events.run(inspection, options);
    return { code: await runCargoQ(inspection.commands, options, this.host()), cargoQ: true };
  }
}
