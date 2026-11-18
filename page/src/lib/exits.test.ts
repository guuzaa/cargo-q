import { afterEach, beforeAll, describe, expect, it, vi } from 'vitest';
import { exits } from './exits';
import { makePage, parser, settle, type } from '../scripts/testing';

beforeAll(async () => {
  // The parser is a wasm module: it is loaded before the clock is faked.
  await parser();
});

afterEach(() => {
  vi.useRealTimers();
});

describe('the exit codes the page promises', () => {
  for (const row of exits) {
    it(`leaves ${row.code} behind for \`${row.line}\``, async () => {
      vi.useFakeTimers();
      const page = makePage();
      const exited = page.nextExit();
      await type(page, row.line);
      if (row.code === 130) {
        // The run never ends on its own: it is stopped with ^C.
        await settle(200);
        page.terminal.send('\x03');
      }
      await settle();
      expect(await exited).toBe(row.code);
    });
  }
});
