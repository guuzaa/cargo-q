import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    // The page is a DOM: the split under the terminal is drawn into one.
    environment: 'happy-dom',
    include: ['tests/unit/**/*.test.ts'],
  },
});
