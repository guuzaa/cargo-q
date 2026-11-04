/**
 * The split, drawn under the terminal.
 *
 * While a line is typed, it shows what cargo-q's parser makes of it: its own
 * options, then the commands, in order. While the line runs, the same pills
 * light up as each command starts, passes, fails or is skipped.
 */

import type { Analysis } from './analyze';
import type { TaskState } from './simulate';
import type { Inspection } from './wasm';

interface Item {
  key: string;
  className: string;
  html: string;
  index?: number;
  run?: string;
}

const escape = (text: string) =>
  text.replace(/[&<>"']/g, (c) => `&#${c.charCodeAt(0)};`);

const ICONS: Record<TaskState | 'pending', string> = {
  pending: '',
  running: '<span class="pill-spin" aria-hidden="true"></span>',
  ok: '<svg class="pill-icon" viewBox="0 0 16 16" aria-hidden="true"><path d="M3.5 8.5l3 3 6-7" /></svg>',
  fail: '<svg class="pill-icon" viewBox="0 0 16 16" aria-hidden="true"><path d="M4 4l8 8M12 4l-8 8" /></svg>',
  skipped: '<svg class="pill-icon" viewBox="0 0 16 16" aria-hidden="true"><path d="M3 8h10" /></svg>',
};

export class SplitView {
  private running = false;
  /** Whether the pills still show the last run. */
  private showingRun = false;
  private rounds = 1;
  private round = 0;

  constructor(
    private readonly root: HTMLElement,
    private readonly status: HTMLElement,
  ) {}

  input(analysis: Analysis): void {
    if (this.running) return;
    if (this.showingRun && analysis.line.trim() === '') return;
    this.showingRun = false;
    this.root.dataset.mode = 'edit';
    this.status.textContent = '';
    const focus = analysis.focus;
    if (!focus) {
      this.draw(analysis.line.trim() === '' ? [this.hint()] : []);
      return;
    }
    this.draw(this.items(focus.inspection));
  }

  run(inspection: Inspection): void {
    this.running = true;
    this.showingRun = true;
    this.rounds = inspection.settings.parallel ? 1 : inspection.settings.repeat;
    this.round = 0;
    this.root.dataset.mode = 'run';
    this.draw(this.items(inspection));
    for (const pill of this.pills()) this.setState(pill, 'pending');
    this.updateRound();
  }

  task(index: number, state: TaskState, round: number): void {
    const pill = this.pills()[index];
    if (!pill) return;
    if (round !== this.round) {
      // A new round: every pill starts again.
      this.round = round;
      for (const other of this.pills()) this.setState(other, 'pending');
      this.updateRound();
    }
    this.setState(pill, state);
  }

  done(): void {
    this.running = false;
    for (const pill of this.pills()) if (pill.dataset.state === 'pending') this.setState(pill, 'skipped');
  }

  // ---------------------------------------------------------------------

  private hint(): Item {
    return {
      key: 'hint',
      className: 'split-hint',
      html: '<span class="kbd">type</span> <span aria-hidden="true">→</span> see the split',
    };
  }

  private items(inspection: Inspection): Item[] {
    if (inspection.help) return [{ key: 'help', className: 'pill pill-option', html: '--help' }];
    if (inspection.version) {
      return [{ key: 'version', className: 'pill pill-option', html: `v${escape(inspection.version)}` }];
    }
    if (inspection.error) return this.errorItems(inspection.error);

    const items: Item[] = [];
    const { settings } = inspection;
    // The options are the parser's own spelling; `--dry-run` and `--repeat`
    // get a pill of their own.
    for (const option of inspection.options) {
      if (option === '--dry-run' || option.startsWith('--repeat')) continue;
      items.push({ key: `o:${option}`, className: 'pill pill-option', html: escape(option) });
    }
    if (settings.dryRun) {
      items.push({ key: 'o:dry', className: 'pill pill-option', html: '--dry-run <span class="pill-note">prints only</span>' });
    }
    const parallel = settings.parallel;
    inspection.commands.forEach((command, index) => {
      if (index > 0) {
        items.push({
          key: `j:${index}:${parallel}`,
          className: `joint ${parallel ? 'joint-parallel' : ''}`,
          html: parallel ? '∥' : '→',
        });
      }
      const args = command.args.length ? ` <span class="pill-args">${escape(command.args.join(' '))}</span>` : '';
      items.push({
        key: `c:${index}:${command.name}:${command.args.join('\u0000')}`,
        className: 'pill pill-command',
        html: `<span class="pill-state"></span><span class="pill-dim">cargo</span> <b>${escape(command.name)}</b>${args}`,
        index,
      });
    });
    if (settings.repeat > 1) {
      items.push({ key: `r:${settings.repeat}`, className: 'pill pill-repeat', html: `×${settings.repeat}` });
    }
    return items;
  }

  private errorItems(error: string): Item[] {
    const lines = error.split('\n').filter((line) => line.trim() !== '');
    if (lines.some((line) => line.includes('required arguments were not provided'))) {
      return [{ key: 'need', className: 'split-hint', html: 'a command, please <span class="kbd">check</span> <span class="kbd">test</span> …' }];
    }
    const message = lines[0].replace(/^error:\s*/, '');
    const items: Item[] = [{ key: `e:${message}`, className: 'pill pill-error', html: escape(message) }];
    for (const line of lines.slice(1)) {
      const fix = line.match(/^help: [^:]+: (cargo q .*)$/);
      if (fix) {
        items.push({
          key: `f:${fix[1]}`,
          className: 'pill pill-fix',
          html: `<span class="pill-dim">fix</span> ${escape(fix[1])}`,
          run: fix[1],
        });
      }
    }
    return items;
  }

  private pills(): HTMLElement[] {
    return [...this.root.querySelectorAll<HTMLElement>('[data-index]:not(.leaving)')].sort(
      (a, b) => Number(a.dataset.index) - Number(b.dataset.index),
    );
  }

  private setState(pill: HTMLElement, state: TaskState | 'pending'): void {
    if (pill.dataset.state === state) return;
    pill.dataset.state = state;
    const slot = pill.querySelector('.pill-state');
    if (slot) slot.innerHTML = ICONS[state];
  }

  private updateRound(): void {
    this.status.textContent = this.rounds > 1 ? `round ${this.round + 1}/${this.rounds}` : '';
  }

  /** Draw the items, keeping the pills that did not change. */
  private draw(items: Item[]): void {
    const existing = new Map<string, HTMLElement>();
    for (const child of [...this.root.children] as HTMLElement[]) {
      if (child.classList.contains('leaving')) continue;
      existing.set(child.dataset.key ?? '', child);
    }

    const live = (node: Element | null): Element | null => {
      let at = node;
      while (at && at.classList.contains('leaving')) at = at.nextElementSibling;
      return at;
    };
    let before = live(this.root.firstElementChild);
    items.forEach((item, order) => {
      let element = existing.get(item.key);
      if (element) {
        existing.delete(item.key);
      } else {
        element = document.createElement(item.run ? 'button' : 'span');
        element.dataset.key = item.key;
        element.className = `${item.className} entering`;
        element.innerHTML = item.html;
        if (item.index !== undefined) element.dataset.index = String(item.index);
        if (item.run) {
          element.dataset.run = item.run;
          element.setAttribute('type', 'button');
        }
        element.style.setProperty('--order', String(order));
        element.addEventListener('animationend', () => element?.classList.remove('entering'), { once: true });
      }
      if (element === before) before = live(before.nextElementSibling);
      else this.root.insertBefore(element, before);
    });

    for (const stale of existing.values()) {
      stale.classList.add('leaving');
      stale.removeAttribute('data-index');
      setTimeout(() => stale.remove(), 180);
    }
  }
}
