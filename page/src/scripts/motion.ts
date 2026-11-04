/**
 * The small motions of the page: things that appear as they scroll in, a
 * light that follows the pointer, copy buttons, sparks and rolling digits.
 */

import { PALETTE } from './palette';

const reduced = () => window.matchMedia('(prefers-reduced-motion: reduce)').matches;

/** `[data-reveal]` fades up once it scrolls into view; `--i` staggers it. */
export function reveal(): void {
  const elements = document.querySelectorAll<HTMLElement>('[data-reveal]');
  if (reduced() || !('IntersectionObserver' in window)) {
    elements.forEach((el) => el.setAttribute('data-shown', ''));
    return;
  }
  const observer = new IntersectionObserver(
    (entries) => {
      for (const entry of entries) {
        if (!entry.isIntersecting) continue;
        entry.target.setAttribute('data-shown', '');
        observer.unobserve(entry.target);
      }
    },
    { rootMargin: '0px 0px -12% 0px' },
  );
  elements.forEach((el) => observer.observe(el));
}

/** The pointer position, as `--mx`/`--my` on the page and on every `[data-glow]`. */
export function spotlight(): void {
  let frame = 0;
  let x = 0;
  let y = 0;
  window.addEventListener(
    'pointermove',
    (event) => {
      x = event.clientX;
      y = event.clientY;
      if (frame) return;
      frame = requestAnimationFrame(() => {
        frame = 0;
        document.documentElement.style.setProperty('--mx', `${x}px`);
        document.documentElement.style.setProperty('--my', `${y}px`);
        for (const el of document.querySelectorAll<HTMLElement>('[data-glow]')) {
          const box = el.getBoundingClientRect();
          el.style.setProperty('--gx', `${x - box.left}px`);
          el.style.setProperty('--gy', `${y - box.top}px`);
        }
      });
    },
    { passive: true },
  );
}

/** `[data-copy]` buttons copy their text and say so. */
export function copyButtons(): void {
  document.addEventListener('click', async (event) => {
    const button = (event.target as Element | null)?.closest<HTMLElement>('[data-copy]');
    if (!button) return;
    try {
      await navigator.clipboard.writeText(button.dataset.copy ?? '');
      button.dataset.copied = '';
      if (!reduced()) burst(button, 10);
      setTimeout(() => delete button.dataset.copied, 1600);
    } catch {
      // Without a clipboard the text is still there to select.
    }
  });
}

/** A few sparks out of an element. */
export function burst(from: HTMLElement, count = 14): void {
  const box = from.getBoundingClientRect();
  const cx = box.left + box.width / 2;
  const cy = box.top + box.height / 2;
  const colors = [PALETTE.status.ok, PALETTE.rust[500], PALETTE.status.warn, PALETTE.rust[300], PALETTE.sand[50]];
  for (let i = 0; i < count; i += 1) {
    const spark = document.createElement('span');
    spark.className = 'spark';
    spark.style.left = `${cx}px`;
    spark.style.top = `${cy}px`;
    spark.style.background = colors[i % colors.length];
    document.body.appendChild(spark);
    const angle = (Math.PI * 2 * i) / count + Math.random() * 0.4;
    const distance = 26 + Math.random() * 34;
    spark
      .animate(
        [
          { transform: 'translate(-50%, -50%) scale(1)', opacity: 1 },
          {
            transform: `translate(calc(-50% + ${Math.cos(angle) * distance}px), calc(-50% + ${Math.sin(angle) * distance}px)) scale(0.2)`,
            opacity: 0,
          },
        ],
        { duration: 520 + Math.random() * 260, easing: 'cubic-bezier(.2,.7,.3,1)' },
      )
      .finished.then(() => spark.remove(), () => spark.remove());
  }
}

/**
 * `[data-odometer="101"]` rolls its digits to that number when it scrolls
 * into view, and again when its row is hovered.
 */
export function odometers(): void {
  const all = document.querySelectorAll<HTMLElement>('[data-odometer]');
  for (const el of all) {
    const value = el.dataset.odometer ?? '0';
    el.textContent = '';
    el.setAttribute('aria-label', value);
    for (const digit of value) {
      const column = document.createElement('span');
      column.className = 'odo-col';
      column.setAttribute('aria-hidden', 'true');
      const strip = document.createElement('span');
      strip.className = 'odo-strip';
      strip.innerHTML = Array.from({ length: 10 }, (_, n) => `<span>${n}</span>`).join('');
      strip.style.setProperty('--d', digit);
      column.appendChild(strip);
      el.appendChild(column);
    }
  }
  if (reduced()) {
    all.forEach((el) => el.setAttribute('data-rolled', ''));
    return;
  }
  const observer = new IntersectionObserver((entries) => {
    for (const entry of entries) {
      if (!entry.isIntersecting) continue;
      const el = entry.target as HTMLElement;
      setTimeout(() => el.setAttribute('data-rolled', ''), Number(el.dataset.delay ?? 0));
      observer.unobserve(el);
    }
  }, { threshold: 0.6 });
  all.forEach((el) => observer.observe(el));
}

/** `[data-latest-version]` gets the version on crates.io, from the worker. */
export async function latestVersion(): Promise<void> {
  const elements = document.querySelectorAll<HTMLElement>('[data-latest-version]');
  if (elements.length === 0) return;
  try {
    const response = await fetch('/api/version');
    if (!response.ok) return;
    const { version } = (await response.json()) as { version?: string };
    if (!version) return;
    for (const el of elements) {
      el.textContent = `v${version}`;
      el.removeAttribute('hidden');
    }
  } catch {
    // `astro dev` has no worker; the page is fine without the version.
  }
}
