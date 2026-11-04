/**
 * How the rest of the page asks the terminal to run a line.
 *
 * Any element with `data-run="cargo q …"` types that line into the terminal
 * when clicked; scripts can call `requestRun` directly.
 */

const EVENT = 'cq:run';

export function requestRun(line: string): void {
  window.dispatchEvent(new CustomEvent<string>(EVENT, { detail: line }));
}

export function onRunRequest(handler: (line: string) => void): void {
  window.addEventListener(EVENT, (event) => handler((event as CustomEvent<string>).detail));
}

/** Wire every `[data-run]` element on the page, now and later. */
export function wireRunButtons(): void {
  document.addEventListener('click', (event) => {
    const target = (event.target as Element | null)?.closest<HTMLElement>('[data-run]');
    if (!target) return;
    event.preventDefault();
    requestRun(target.dataset.run ?? '');
  });
}
