/**
 * Splitting a line the way a shell would, before any program sees it.
 *
 * Quotes group words, a backslash takes the next character literally, and
 * `&&`, `||` and `;` separate commands. Every token remembers where it was
 * written, so the shell can color the line as it is typed.
 */

export type Operator = '&&' | '||' | ';';

export interface Token {
  /** The token, as the program receives it. */
  text: string;
  /** Where it starts and ends in the line. */
  start: number;
  end: number;
  /** Set when the token is an operator rather than a word. */
  op?: Operator;
}

export interface Tokenized {
  tokens: Token[];
  /** A quote that is still open at the end of the line. */
  open: boolean;
}

export function tokenize(line: string): Tokenized {
  const tokens: Token[] = [];
  let text = '';
  let start = -1;
  let quote: '"' | "'" | null = null;

  const push = (end: number) => {
    if (start < 0) return;
    tokens.push({ text, start, end });
    text = '';
    start = -1;
  };

  for (let i = 0; i < line.length; i += 1) {
    const c = line[i];
    if (quote) {
      if (c === quote) quote = null;
      else if (c === '\\' && quote === '"' && i + 1 < line.length) text += line[++i];
      else text += c;
      continue;
    }
    const pair = line.slice(i, i + 2);
    if (pair === '&&' || pair === '||' || c === ';') {
      push(i);
      const op = (c === ';' ? ';' : pair) as Operator;
      tokens.push({ text: op, start: i, end: i + op.length, op });
      i += op.length - 1;
      continue;
    }
    if (/\s/.test(c)) {
      push(i);
      continue;
    }
    if (start < 0) start = i;
    if (c === '"' || c === "'") quote = c;
    else if (c === '\\' && i + 1 < line.length) text += line[++i];
    else text += c;
  }
  push(line.length);
  return { tokens, open: quote !== null };
}

/** The words of a line, cut into the commands between its operators. */
export function segments(tokens: Token[]): { words: Token[]; op: Operator | null }[] {
  const out: { words: Token[]; op: Operator | null }[] = [{ words: [], op: null }];
  for (const token of tokens) {
    if (token.op) {
      out[out.length - 1].op = token.op;
      out.push({ words: [], op: null });
    } else {
      out[out.length - 1].words.push(token);
    }
  }
  return out;
}

/** Quote a word so that a shell reads it back as the same word. */
export function quote(word: string): string {
  if (word !== '' && /^[\w@%+=:,./-]+$/.test(word)) return word;
  return `"${word.replace(/(["\\$`])/g, '\\$1')}"`;
}
