/**
 * The terminal at the top of the page.
 *
 * A terminal to type in, and a shell that answers with cargo-q's own
 * parser.
 */
import '@xterm/xterm/css/xterm.css';
import { Terminal } from '@xterm/xterm';
import { FitAddon } from '@xterm/addon-fit';
import { Shell } from './shell';

const element = document.getElementById('cargo-q-terminal');

if (element) {
  const terminal = new Terminal({
    convertEol: false,
    cursorBlink: true,
    cursorStyle: 'block',
    fontFamily: "'JetBrains Mono Variable', ui-monospace, monospace",
    fontSize: 13,
    lineHeight: 1.35,
    scrollback: 200,
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

  const shell = new Shell(terminal, []);

  // Not awaited at the top level: the scripts bundled after this one (the
  // recordings) must not wait for the parser, nor stop when it fails.
  void (async () => {
    try {
      await shell.setNames();
      await shell.setVersion();
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      terminal.writeln(`\x1b[38;2;255;123;114m${message}\x1b[0m`);
    }
    shell.start();

    terminal.onData((data) => shell.onData(data));

    // The examples below the terminal type their line into it.
    window.addEventListener('cargo-q:run', (event) => {
      const line = (event as CustomEvent<string>).detail;
      void shell.typeAndRun(line);
    });
  })();
}