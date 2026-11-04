/**
 * The terminal at the top of the page, and everything that reacts to it:
 * the split view, the exit-code badge, the window's glow and the presets.
 */

import { Terminal } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import '@xterm/xterm/css/xterm.css';
import { onRunRequest, wireRunButtons } from './bus';
import { Shell } from './shell';
import { SplitView } from './split-view';
import { parser } from './wasm';
import { burst } from './motion';

const host = document.querySelector<HTMLElement>('#terminal');
const windowEl = document.querySelector<HTMLElement>('#term-window');
const splitEl = document.querySelector<HTMLElement>('#split');
const roundEl = document.querySelector<HTMLElement>('#split-round');
const exitEl = document.querySelector<HTMLElement>('#exit-code');
const wasmEl = document.querySelector<HTMLElement>('#wasm-badge');

const reduced = window.matchMedia('(prefers-reduced-motion: reduce)').matches;

const PRESETS = [
  'cargo q fmt clippy check test',
  'cargo q -k test build',
  'cargo q -p check clippy doc',
  'cargo q --repeat 3 check',
  'cargo q -n build --features test',
  'cargo q build --features feature1',
  'cargo q -v run',
  'cargo q watch',
  'cargo q --help',
];

if (host && windowEl && splitEl && roundEl && exitEl) {
  const term = new Terminal({
    fontFamily: "'JetBrains Mono Variable', ui-monospace, Menlo, monospace",
    fontSize: window.innerWidth < 640 ? 11.5 : 13.5,
    lineHeight: 1.25,
    cursorBlink: true,
    cursorStyle: 'bar',
    cursorWidth: 2,
    allowProposedApi: false,
    scrollback: 2000,
    theme: {
      background: '#00000000',
      foreground: '#f0e9df',
      cursor: '#f26b21',
      cursorAccent: '#0e0c0a',
      selectionBackground: '#9c3a1288',
      black: '#16130f',
      red: '#ff7b72',
      green: '#7ee787',
      yellow: '#e3b341',
      blue: '#79c0ff',
      magenta: '#d2a8ff',
      cyan: '#56d4dd',
      white: '#f0e9df',
      brightBlack: '#5c5449',
      brightRed: '#ffa198',
      brightGreen: '#a5f0ab',
      brightYellow: '#f2cc60',
      brightBlue: '#a5d6ff',
      brightMagenta: '#e2c5ff',
      brightCyan: '#8be9f0',
      brightWhite: '#faf6f0',
    },
    allowTransparency: true,
  });
  const fit = new FitAddon();
  term.loadAddon(fit);

  const view = new SplitView(splitEl, roundEl);
  const ready = parser();

  const setExit = (code: number) => {
    exitEl.textContent = String(code);
    exitEl.dataset.kind = code === 0 ? 'ok' : code === 130 ? 'int' : 'fail';
    exitEl.classList.remove('pop');
    void exitEl.offsetWidth;
    exitEl.classList.add('pop');
  };

  const flash = (state: string) => {
    windowEl.dataset.state = state;
    if (state === 'busy') return;
    setTimeout(() => {
      if (windowEl.dataset.state === state) delete windowEl.dataset.state;
    }, 1400);
  };

  const shell = new Shell(
    term,
    ready,
    {
      input: (analysis) => view.input(analysis),
      run: (inspection) => {
        flash('busy');
        view.run(inspection);
      },
      task: (index, state, round) => view.task(index, state, round),
      exit: (code, cargoQ) => {
        setExit(code);
        if (!cargoQ) return;
        view.done();
        flash(code === 0 ? 'ok' : code === 130 ? 'int' : 'fail');
        if (code === 0 && !reduced) burst(exitEl);
      },
      busy: (busy) => {
        if (!busy && windowEl.dataset.state === 'busy') delete windowEl.dataset.state;
        document.body.toggleAttribute('data-running', busy);
      },
    },
    PRESETS,
  );

  // The font has to be there before the terminal measures its cells.
  void document.fonts.load("13px 'JetBrains Mono Variable'").finally(() => {
    term.open(host);
    fit.fit();
    new ResizeObserver(() => fit.fit()).observe(host);
    void ready.then(
      (p) => {
        if (wasmEl) {
          wasmEl.dataset.ready = '';
          wasmEl.title = `cargo-q ${p.version}'s parser, compiled to WebAssembly (${Math.round(p.size / 1024)} KB)`;
        }
        start(p.version);
      },
      () => start(null),
    );
  });

  let started = false;
  const start = (version: string | null) => {
    if (started) return;
    started = true;
    const D = '\x1b[38;2;139;129;115m';
    const O = '\x1b[38;2;242;107;33m';
    const R = '\x1b[0m';
    const v = version ? ` ${version}` : '';
    shell.start(
      `${O}cargo-q${v}${R} ${D}· parser: wasm · cargo: simulated · type ${R}help${D} or press ${R}tab${R}\r\n\r\n`,
    );
    autoplay();
  };

  // Play the first line once the terminal is in view, unless someone is
  // already typing.
  let touched = false;
  host.addEventListener('keydown', () => (touched = true), { capture: true });
  host.addEventListener('pointerdown', () => (touched = true));
  const autoplay = () => {
    const observer = new IntersectionObserver(
      (entries) => {
        if (!entries.some((entry) => entry.isIntersecting)) return;
        observer.disconnect();
        setTimeout(() => {
          if (!touched) void shell.type(PRESETS[0], { instant: reduced });
        }, 700);
      },
      { threshold: 0.5 },
    );
    observer.observe(host);
  };

  wireRunButtons();
  onRunRequest((line) => {
    touched = true;
    const top = windowEl.getBoundingClientRect().top;
    if (top < 0 || top > window.innerHeight * 0.5) {
      windowEl.scrollIntoView({ behavior: reduced ? 'auto' : 'smooth', block: 'center' });
    }
    shell.focus();
    void shell.type(line, { instant: reduced });
  });

  windowEl.addEventListener('click', (event) => {
    if (!(event.target as Element).closest('button')) shell.focus();
  });
}
