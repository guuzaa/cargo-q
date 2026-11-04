# The landing page of cargo-q

The site at <https://cargo-q.paulden.site>: Astro, Tailwind and a Worker, deployed on
Cloudflare.

## What is here

| Path | What it is |
| --- | --- |
| `src/` | The site: Astro components, the styles, and the scripts that run in the browser. |
| `src/scripts/shell.ts` | The shell in the terminal at the top of the page: it takes what is typed, asks cargo-q's parser what it means, and runs the simulation. |
| `src/scripts/palette.ts` | The colours of the page, shared by the stylesheet and by the scripts that paint the terminal. |
| `src/scripts/simulate.ts` | The rehearsal of a run: the progress display, the failures, the summary and the exit code. `cargo test` fails in it on purpose. |
| `src/worker.ts` | The Worker: it answers `/api/version` and hands everything else to the static assets. |
| `wasm/` | A small Rust crate that compiles cargo-q's parser to WebAssembly. |
| `public/demo/` | Recordings of real `cargo q` runs, replayed in the page. |
| `og/card.svg` | The social card, the source of `public/og.png` (`og:image`). |

The parser in `wasm/` is cargo-q's own code: the page asks it what a command
line means, so the page and a real run always agree. The crate is a `cdylib`
with a plain C ABI, so the page needs no wasm toolchain of its own.

The terminal takes input. What is typed is parsed by cargo-q's parser — the
split is the real one — and the cargo commands are then simulated, so
nothing runs on the visitor's machine. `cargo q --help` prints the real help
text, and `echo $?` prints the exit code of the last run.

## Working on it

```bash
pnpm install
pnpm dev            # the site on http://localhost:4321
pnpm build          # the parser, then the site, into dist/
pnpm check          # the colours of the page, and the types of the site and the worker
pnpm preview        # the built site
```

`pnpm build` compiles the parser first (`pnpm build:wasm`), which needs a
Rust toolchain with the `wasm32-unknown-unknown` target:

```bash
rustup target add wasm32-unknown-unknown
```

The site needs Node 22 or later. On this machine `node` on the `PATH` is bun;
wrangler does not run under bun, so use the real one:

```bash
PATH=/usr/bin:$PATH pnpm exec wrangler dev
```

## Deploying

`wrangler deploy` publishes the assets and the Worker to Cloudflare. It
needs an API token with the Workers and Pages permissions:

```bash
export CLOUDFLARE_API_TOKEN=...
export CLOUDFLARE_ACCOUNT_ID=...
pnpm deploy
```

The workflow in `.github/workflows/page.yml` does this on every push to
`main` that touches the site or the parser.

## The recordings

`public/demo/*.ansi` are the raw bytes of a real run, recorded from a
terminal. To record a new one, run cargo-q in a pty and keep the output:

```bash
script -qec "stty cols 100 rows 30; cargo q fmt clippy check test" /tmp/run.ansi
```

Take the bytes between the `Script started on` and `Script done on` lines,
and they are what the page replays.

## The social card

`public/og.png` is `og/card.svg` rendered at 1200×630. After a change to the
SVG, render it again with any SVG rasterizer, for example:

```bash
rsvg-convert -w 1200 -h 630 og/card.svg -o public/og.png
```

The renderer takes the font from the system: the card names JetBrains Mono
first and falls back to DejaVu Sans Mono. The published card is in the
fallback — the machine that rendered it had no JetBrains Mono — so a machine
that has JetBrains Mono renders a card in another typeface.
