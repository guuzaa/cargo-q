import { beforeAll, describe, expect, it } from 'vitest';
import { buildParser, dryRun } from './parser';
import { parser } from '../scripts/testing';

beforeAll(async () => {
  // The parser is a wasm module: it is loaded before the clock is faked.
  await parser();
});

describe("cargo-q's parser", () => {
  it('splits a line into the commands it would run', async () => {
    const parser = await buildParser();
    const inspection = parser.inspect(['cargo', 'q', 'fmt', 'check']);
    expect(inspection.commands).toEqual([
      { name: 'fmt', args: [] },
      { name: 'check', args: [] },
    ]);
    expect(inspection.error).toBeNull();
    expect(inspection.exitCode).toBeNull();
  });

  it('keeps the arguments of a command', async () => {
    const parser = await buildParser();
    const inspection = parser.inspect(['cargo', 'q', 'build', '--features=foo']);
    expect(inspection.commands[0]).toEqual({ name: 'build', args: ['--features=foo'] });
  });

  it('rejects a word that is not a cargo subcommand', async () => {
    const parser = await buildParser();
    const inspection = parser.inspect(['cargo', 'q', 'build', '--features', 'feature1']);
    expect(inspection.exitCode).toBe(1);
    expect(inspection.error).toContain('is not a cargo subcommand');
  });

  it('rejects options that cannot go together', async () => {
    const parser = await buildParser();
    const inspection = parser.inspect(['cargo', 'q', '-p', '--repeat', '2', 'check']);
    expect(inspection.exitCode).toBe(2);
  });

  it('prints the plan of a line', async () => {
    const plan = await dryRun('cargo q -n build --features=foo');
    expect(plan.join('\n')).toContain('cargo build --features=foo');
  });

  it('prints the help it would print', async () => {
    const parser = await buildParser();
    const inspection = parser.inspect(['cargo', 'q', '--help']);
    expect(inspection.help).toContain('Usage: cargo-q [OPTIONS] <COMMANDS>...');
    expect(inspection.exitCode).toBe(0);
  });

  it('says the version it was built from', async () => {
    const parser = await buildParser();
    expect(parser.version).toMatch(/^\d+\.\d+\.\d+/);
  });
});
