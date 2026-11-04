/**
 * The same wasm parser, at build time.
 *
 * The page's static parts (the colored command lines, the dry-run previews)
 * are rendered by cargo-q's own parser when the site is built, so they need
 * no script in the browser and cannot drift from what cargo-q does.
 * `pnpm build:wasm` has to have run first; `pnpm build` does that.
 */

import { readFile } from 'node:fs/promises';
import { join } from 'node:path';
import { fromBytes, type Parser } from '../scripts/wasm';

let loading: Promise<Parser> | undefined;

async function load(): Promise<Parser> {
  const path = join(process.cwd(), 'public', 'wasm', 'cargo-q.wasm');
  let bytes: Buffer;
  try {
    bytes = await readFile(path);
  } catch {
    throw new Error(`${path} is missing: run \`pnpm build:wasm\` first`);
  }
  return fromBytes(new Uint8Array(bytes));
}

export function buildParser(): Promise<Parser> {
  loading ??= load();
  return loading;
}

/** What `cargo q -n …` would print for a line, or the error it would give. */
export async function dryRun(line: string): Promise<string[]> {
  const parser = await buildParser();
  const inspection = parser.inspect(line.split(/\s+/).filter(Boolean));
  const text = inspection.plan ?? inspection.error ?? '';
  return text.split('\n').filter(Boolean).slice(0, 4);
}
