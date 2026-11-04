/**
 * Splitting a command line into the tokens a shell would hand to a program.
 *
 * A shell splits a line before any program sees it, so the page has to do
 * the same: the tokens it shows are the tokens cargo-q would be given.
 */

/** One token of a command line. */
export interface ShellToken {
  /** The token, as the program would receive it. */
  text: string;
  /** The token, as it was written. */
  raw: string;
  /** Whether the token was written in quotes. */
  quoted: boolean;
}

/** The tokens of a command line, or the mistake that stopped the split. */
export interface Tokenized {
  tokens: ShellToken[];
  error: string | null;
}

/**
 * Split a command line into tokens.
 *
 * Quotes group words, and a backslash takes the next character literally.
 */
export function tokenize(line: string): Tokenized {
  const tokens: ShellToken[] = [];

  let text = '';
  let raw = '';
  let quoted = false;
  let started = false;
  let quote: '"' | "'" | null = null;

  const push = () => {
    if (!started) return;
    tokens.push({ text, raw, quoted });
    text = '';
    raw = '';
    quoted = false;
    started = false;
  };

  for (let index = 0; index < line.length; index += 1) {
    const character = line[index];

    if (quote) {
      if (character === quote) {
        quote = null;
        continue;
      }
      if (character === '\\' && quote === '"' && index + 1 < line.length) {
        index += 1;
        text += line[index];
        raw += `\\${line[index]}`;
        continue;
      }
      text += character;
      raw += character;
      continue;
    }

    if (character === '"' || character === "'") {
      quote = character;
      quoted = true;
      started = true;
      raw += character;
      continue;
    }

    if (character === '\\' && index + 1 < line.length) {
      index += 1;
      text += line[index];
      raw += `\\${line[index]}`;
      started = true;
      continue;
    }

    if (/\s/.test(character)) {
      push();
      continue;
    }

    text += character;
    raw += character;
    started = true;
  }

  if (quote) {
    return { tokens, error: 'the quote before the end is never closed' };
  }

  push();
  return { tokens, error: null };
}

/**
 * The arguments that follow `cargo q`.
 *
 * The program name goes, and so does every `q`: cargo passes the name of
 * the subcommand along, and cargo-q drops it again.
 */
export function cargoQArgs(tokens: ShellToken[]): ShellToken[] {
  const args = tokens[0]?.text === 'cargo' ? tokens.slice(1) : tokens.slice(0);
  return args.filter((token) => token.text !== 'q');
}

/**
 * The error a command line gets when it does not name the subcommand.
 *
 * `cargo q` and `cargo    q` are the same thing — how many spaces are
 * between them does not matter — but `cargo` on its own does not name
 * cargo-q, and neither does a bare list of commands.
 */
export function checkCargoQ(tokens: ShellToken[]): string | null {
  if (tokens[0]?.text === 'cargo' && tokens[1]?.text === 'q') return null;

  return 'error: the command line must start with "cargo q"\nhelp: try: cargo q fmt clippy check test';
}
