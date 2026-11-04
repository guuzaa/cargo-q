/**
 * The copy buttons.
 *
 * A button with `data-copy` copies the text it carries, and says so for a
 * moment afterwards.
 */
document.addEventListener('click', async (event) => {
  const target = event.target as HTMLElement | null;
  const button = target?.closest<HTMLElement>('[data-copy]');
  if (!button) return;

  const text = button.dataset.copy ?? '';
  const label = button.textContent;

  try {
    await navigator.clipboard.writeText(text);
  } catch {
    // The clipboard may be blocked: leave the text where it can be taken by
    // hand instead of pretending it was copied.
    return;
  }

  button.textContent = 'copied';
  button.classList.add('text-rust-400');
  window.setTimeout(() => {
    button.textContent = label;
    button.classList.remove('text-rust-400');
  }, 1400);
});
