/**
 * The colours of the page, in one place.
 *
 * The same values live in `src/styles/global.css`, under `@theme`: the page
 * paints itself with them, and the terminal has to be built with them the
 * moment the page loads. `pnpm check` makes sure the two have not drifted
 * apart, so a colour is changed here and there.
 */

export const PALETTE = {
  /** The warm dark of a terminal at night. */
  ink: {
    950: '#090807',
    900: '#0e0c0a',
    850: '#14110e',
    800: '#1d1915',
    700: '#2b251e',
    600: '#3d352b',
  },
  /** The text on it. */
  sand: {
    50: '#faf6f0',
    100: '#f0e9df',
    300: '#c3b8a8',
    500: '#8b8173',
    600: '#847a6b',
  },
  /** Rust: the brand. */
  rust: {
    300: '#ffb08a',
    400: '#ff8f5e',
    500: '#f26b21',
    600: '#d1521a',
    700: '#9c3a12',
  },
  /** What a run says about itself. */
  status: {
    ok: '#84d98a',
    warn: '#e3b341',
    fail: '#ff8f7d',
    info: '#8ab8e0',
  },
  /** The rest of the terminal's palette, for what cargo prints. */
  ansi: {
    magenta: '#d4a0e0',
    cyan: '#6fc9c0',
    brightRed: '#ffa89f',
    brightGreen: '#a9edaa',
    brightYellow: '#f2cc60',
    brightBlue: '#a9c9ec',
    brightMagenta: '#e6bde8',
    brightCyan: '#95d8ce',
  },
} as const;

/** A colour, as the terminal's own output writes it. */
export const ansi = (hex: string): string => {
  const n = parseInt(hex.slice(1), 16);
  return `\x1b[38;2;${n >> 16};${(n >> 8) & 255};${n & 255}m`;
};
