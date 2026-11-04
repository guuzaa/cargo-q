import { execFileSync } from 'node:child_process';
import { copyFileSync, mkdirSync, statSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = dirname(dirname(fileURLToPath(import.meta.url)));
const manifest = join(root, 'wasm', 'Cargo.toml');
const target = 'wasm32-unknown-unknown';

// The parser is cargo-q's own code, compiled to WebAssembly: the page and a
// real run agree on what a command line means, because they are the same
// code that decides it.
execFileSync(
  'cargo',
  ['build', '--manifest-path', manifest, '--target', target, '--release'],
  { stdio: 'inherit' },
);

const built = join(root, 'wasm', 'target', target, 'release', 'cargo_q_wasm.wasm');
const out = join(root, 'public', 'wasm', 'cargo-q.wasm');

mkdirSync(dirname(out), { recursive: true });
copyFileSync(built, out);

console.log(`wasm: ${out} (${statSync(out).size} bytes)`);
