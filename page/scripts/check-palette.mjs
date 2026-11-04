/**
 * The palette lives in two places: `src/styles/global.css`, which paints the
 * page, and `src/scripts/palette.ts`, which paints the terminal. They have to
 * agree, or the terminal and the page around it would drift apart in colour.
 */
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = dirname(dirname(fileURLToPath(import.meta.url)));
const css = readFileSync(join(root, 'src', 'styles', 'global.css'), 'utf8');
const script = readFileSync(join(root, 'src', 'scripts', 'palette.ts'), 'utf8');

/** The colours of the stylesheet: `--color-<name>: #hex;`. */
const inCss = new Map();
for (const [, name, hex] of css.matchAll(/--color-([a-z0-9-]+):\s*(#[0-9a-f]{6})\s*;/gi)) {
  inCss.set(name, hex.toLowerCase());
}

/** One group of the script: `ink: { 950: '#090807', … }`. */
const group = (name) => {
  const body = script.match(new RegExp(`\\n  ${name}: \\{([\\s\\S]*?)\\n  \\},`));
  if (!body) throw new Error(`palette.ts: no group "${name}"`);
  const out = new Map();
  for (const [, key, hex] of body[1].matchAll(/([a-zA-Z0-9]+):\s*'(#[0-9a-f]{6})'/g)) {
    out.set(key, hex.toLowerCase());
  }
  return out;
};

/** The name a colour of the script has in the stylesheet. */
const name = (group, key) => (group === 'status' ? key : `${group}-${key}`);

const expected = new Map();
for (const g of ['ink', 'sand', 'rust', 'status']) {
  for (const [key, hex] of group(g)) expected.set(name(g, key), hex);
}

let wrong = 0;
for (const [key, hex] of expected) {
  const found = inCss.get(key);
  if (found === hex) continue;
  wrong += 1;
  console.error(`palette: --color-${key} is ${found ?? 'missing'} in global.css, ${hex} in palette.ts`);
}
for (const [key, hex] of inCss) {
  if (expected.has(key)) continue;
  wrong += 1;
  console.error(`palette: --color-${key} (${hex}) is in global.css, but not in palette.ts`);
}

if (wrong) {
  console.error(`palette: ${wrong} colour(s) do not match`);
  process.exit(1);
}
console.log(`palette: ${expected.size} colours match`);
