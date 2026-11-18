// @ts-check
import { defineConfig } from 'astro/config';
import tailwindcss from '@tailwindcss/vite';

// https://astro.build/config
export default defineConfig({
  site: 'https://cargo-q.paulden.site',
  // The toolbar of the development server sits over the page, and the
  // tests click what is under it.
  devToolbar: { enabled: false },
  vite: {
    plugins: [tailwindcss()],
  },
});
