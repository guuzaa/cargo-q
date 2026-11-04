/**
 * The recorded runs.
 *
 * Each `[data-recording]` element gets a terminal that replays what a real
 * run looked like: the frames were recorded from a terminal, so the
 * overprinting progress display comes back as it was.
 */
import '@xterm/xterm/css/xterm.css';
import { Terminal } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';

const reduced = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
const sleep = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

for (const element of document.querySelectorAll<HTMLElement>('[data-recording]')) {
  const source = element.dataset.src;
  if (!source) continue;

  const terminal = new Terminal({
    convertEol: false,
    cursorBlink: false,
    cursorStyle: 'block',
    fontFamily: "'JetBrains Mono Variable', ui-monospace, monospace",
    fontSize: 13,
    lineHeight: 1.35,
    scrollback: 0,
    theme: {
      background: '#100e0c',
      foreground: '#f0e9df',
      cursor: '#f26b21',
      green: '#7ee787',
      red: '#ff7b72',
      yellow: '#ffb08a',
      brightBlack: '#5c5449',
    },
  });

  const fit = new FitAddon();
  terminal.loadAddon(fit);
  terminal.open(element);
  fit.fit();
  window.addEventListener('resize', () => fit.fit());

  const response = await fetch(source);
  const recording = await response.text();

  /** The frames of the recording, each one from the last clear. */
  const frames = recording.split(/(?=\x1b\[J)/);

  async function type(text: string) {
    for (const character of text) {
      terminal.write(character);
      await sleep(reduced ? 0 : 24);
    }
  }

  async function play() {
    terminal.reset();
    fit.fit();

    const command = element.dataset.command;
    if (command) {
      await type(`$ ${command}`);
      terminal.write('\r\n');
    }

    for (const frame of frames) {
      terminal.write(frame);
      await sleep(reduced ? 0 : 420);
    }
  }

  const replay = element.closest('[data-recording-card]')?.querySelector('[data-replay]');
  replay?.addEventListener('click', () => void play());

  await play();
}
