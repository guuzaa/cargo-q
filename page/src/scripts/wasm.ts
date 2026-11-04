/**
 * The wasm module that answers what a command line means.
 *
 * The module is cargo-q's own parser, compiled to WebAssembly: the split
 * the page shows is the split a run would make.
 */

/** What cargo-q makes of a command line. */
export interface Inspection {
  /** cargo-q's own options, in the order they were given. */
  options: string[];
  /** The commands the line was split into. */
  commands: { name: string; args: string[] }[];
  /** The help text the line asks for, when it asks for any. */
  help: string | null;
  /** The version the line asks for, when it asks for it. */
  version: string | null;
  /** The error the line would report, when it is not one cargo-q accepts. */
  error: string | null;
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

let loading: Promise<Exports> | undefined;

/** The module, loaded once. A load that fails is tried again next time. */
function load(): Promise<Exports> {
  loading ??= fetchModule().catch((error: unknown) => {
    loading = undefined;
    throw error;
  });
  return loading;
}

async function fetchModule(): Promise<Exports> {
  const response = await fetch('/wasm/cargo-q.wasm');
  if (!response.ok) {
    throw new Error(`the parser could not be loaded (${response.status})`);
  }
  const bytes = await response.arrayBuffer();
  const module = await WebAssembly.compile(bytes);
  const instance = new WebAssembly.Instance(module, {});
  return instance.exports as unknown as Exports;
}

/** Write a string into the module's buffer. */
function put(api: Exports, text: string): number {
  const bytes = new TextEncoder().encode(text);
  const pointer = api.cq_input_buffer(bytes.length);
  new Uint8Array(api.memory.buffer, pointer, bytes.length).set(bytes);
  return bytes.length;
}

/** Read a string out of the module's memory. */
function get(api: Exports, pointer: number, length: number): string {
  return new TextDecoder().decode(new Uint8Array(api.memory.buffer, pointer, length));
}

/**
 * What a command line means.
 *
 * The arguments are the ones that follow `cargo q`. The names are the cargo
 * subcommands the caller knows about, besides the well-known ones.
 */
export async function inspect(args: string[], names: string[]): Promise<Inspection> {
  const api = await load();

  const namesLength = put(api, names.join('\0'));
  api.cq_set_names(namesLength);

  // Each token ends in a NUL, so an empty token (`""`) is still a token.
  const tokensLength = put(api, args.map((arg) => `${arg}\0`).join(''));
  const pointer = api.cq_inspect(tokensLength);
  const length = api.cq_result_len();

  return JSON.parse(get(api, pointer, length)) as Inspection;
}

/** The cargo subcommand names cargo-q knows without asking cargo. */
export async function wellKnown(): Promise<string[]> {
  const api = await load();
  const pointer = api.cq_well_known();
  const length = api.cq_well_known_len();
  return get(api, pointer, length).split('\0').filter(Boolean);
}

/** The version of cargo-q the parser was built from. */
export async function version(): Promise<string> {
  const api = await load();
  return get(api, api.cq_version(), api.cq_version_len());
}
