/**
 * The version on crates.io, as it is now.
 *
 * Every `[data-latest-version]` element gets `v<version>`, after its
 * `data-prefix` when it has one. The worker answers `/api/version`; without
 * it (`astro dev`, or crates.io not answering) the elements stay empty, and
 * the page is fine without them.
 */
const elements = document.querySelectorAll<HTMLElement>('[data-latest-version]');

if (elements.length > 0) {
  void (async () => {
    try {
      const response = await fetch('/api/version');
      if (!response.ok) return;
      const { version } = (await response.json()) as { version?: string };
      if (!version) return;
      for (const element of elements) {
        element.textContent = `${element.dataset.prefix ?? ''}v${version}`;
      }
    } catch {
      // The page is fine without it.
    }
  })();
}
