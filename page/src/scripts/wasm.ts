/**
 * cargo-q's own parser, compiled to WebAssembly.
 *
 * The split the page shows is the split a real run would make, because it
 * is the same code that makes it. Once loaded, every call is synchronous, so
 * the shell can ask on every keystroke.
 */

/** What cargo-q makes of a command line. */
export interface Inspection {
  /** cargo-q's own options, in the order they were given. */
  options: string[];
  /** The commands the line was split into. */
  commands: Command[];
  /** The help text the line asks for, when it asks for any. */
  help: string | null;
  /** The version the line asks for, when it asks for it. */
  version: string | null;
  /** The error the line would report, when cargo-q rejects it. */
  error: string | null;
  /** The options, as values. */
  settings: Settings;
  /** What `--dry-run` prints for the commands, when the line splits into any. */
  plan: string | null;
  /**
   * The exit code the line ends with right away: 0 for the help and the
   * version, 2 for a line the option parser rejects, 1 for a command it
   * cannot split. `null` when the line would run.
   */
  exitCode: number | null;
}

export interface Settings {
  verbose: boolean;
  parallel: boolean;
  keepGoing: boolean;
  dryRun: boolean;
  noFancy: boolean;
  /** How many rounds the list runs: one without `--repeat`. */
  repeat: number;
}

export interface Command {
  name: string;
  args: string[];
}

interface Exports {
  memory: WebAssembly.Memory;
  cq_input_buffer(length: number): number;
  cq_set_names(length: number): void;
  cq_inspect(length: number): number;
  cq_result_len(): number;
  cq_well_known(): number;
  cq_well_known_len(): number;
  cq_version(): number;
  cq_version_len(): number;
}

/** The third-party subcommands the simulated machine has installed. */
export const EXTRA_NAMES = ['nextest', 'watch', 'deny', 'audit', 'llvm-cov', 'expand', 'udeps', 'machete'];

/** The parser, ready to answer. */
export interface Parser {
  /**
   * What a command line means. The words are the whole line as a shell
   * passes it (`cargo`, `q`, `check`, …): cargo-q drops the program name
   * and every `q` itself.
   */
  inspect(argv: string[]): Inspection;
  /** Every subcommand name the simulated cargo knows. */
  names: string[];
  /** The version of cargo-q the module was built from. */
  version: string;
  /** The size of the module, in bytes. */
  size: number;
}

const encoder = new TextEncoder();
const decoder = new TextDecoder();

let loading: Promise<Parser> | undefined;

/** The parser, loaded once. A load that fails is tried again next time. */
export function parser(): Promise<Parser> {
  loading ??= load().catch((error: unknown) => {
    loading = undefined;
    throw error;
  });
  return loading;
}

async function load(): Promise<Parser> {
  const response = await fetch('/wasm/cargo-q.wasm');
  if (!response.ok) throw new Error(`the parser could not be loaded (${response.status})`);
  return fromBytes(await response.arrayBuffer());
}

/** The parser, from the bytes of the module (the build reads them from disk). */
export async function fromBytes(bytes: ArrayBuffer | Uint8Array<ArrayBuffer>): Promise<Parser> {
  // `compile` and `Instance` rather than `instantiate`: the Workers types the
  // project also loads give `instantiate` a different signature.
  const instance = new WebAssembly.Instance(await WebAssembly.compile(bytes), {});
  const api = instance.exports as unknown as Exports;

  const put = (text: string): number => {
    const data = encoder.encode(text);
    const pointer = api.cq_input_buffer(data.length);
    new Uint8Array(api.memory.buffer, pointer, data.length).set(data);
    return data.length;
  };
  const get = (pointer: number, length: number): string =>
    decoder.decode(new Uint8Array(api.memory.buffer, pointer, length));

  const wellKnown = get(api.cq_well_known(), api.cq_well_known_len()).split('\0').filter(Boolean);
  api.cq_set_names(put(EXTRA_NAMES.join('\0')));

  return {
    inspect(argv) {
      // Each token ends in a NUL, so an empty token (`""`) is still a token.
      const length = put(argv.map((arg) => `${arg}\0`).join(''));
      const pointer = api.cq_inspect(length);
      return JSON.parse(get(pointer, api.cq_result_len())) as Inspection;
    },
    names: [...wellKnown, ...EXTRA_NAMES].sort(),
    version: get(api.cq_version(), api.cq_version_len()),
    size: bytes.byteLength,
  };
}
