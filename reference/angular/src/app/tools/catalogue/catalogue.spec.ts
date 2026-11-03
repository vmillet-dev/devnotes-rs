import { TestBed } from '@angular/core/testing';
import { describe, expect, it } from 'vitest';
import { matchesSearchable } from '@core/services/tools/reference.model';
import {
  KEPT_RECENT_TOOLS,
  MAX_RECENT_TOOLS,
  TOOL_CATEGORIES,
  Tool,
  toolMatches,
} from '@core/services/tools/tool.model';
import { renderTool } from '@testing/tool-harness';
import { TOOLS } from './catalogue';

const [REFERENCES, TOOLS_THAT_ASK] = [
  TOOLS.filter((tool) => tool.category === 'reference'),
  TOOLS.filter((tool) => tool.category !== 'reference'),
];

describe('TOOLS', () => {
  /** « Numéros de contrôle » became « Luhn et IBAN »: the words one typed for it still find it. */
  it.each(['contrôle', 'controle', 'clé', 'checksum', 'check digits'])(
    'finds the check digits by « %s »',
    (word) => {
      const checks = TOOLS.find((tool) => tool.id === 'checks')!;

      expect(toolMatches(word, checks.keywords)).toBe(true);
    },
  );

  /** What the home's search reads of a reference: its rows, built as its table builds them. */
  it.each(REFERENCES.map((tool) => [tool.id, tool] as const))(
    '%s hands its rows to the search, each with a key of its own and a name',
    async (_, tool) => {
      expect(tool.reference).toBeDefined();
      const source = await tool.reference!();

      for (const locale of ['fr', 'en'] as const) {
        const rows = source.rows((await source.words[locale]()).default, (key) => key);
        expect(rows.length).toBeGreaterThan(10);
        expect(new Set(rows.map((row) => row.key)).size).toBe(rows.length);
        expect(rows.every((row) => source.title(row).trim() !== '')).toBe(true);
        expect(rows.filter((row) => source.line(row).trim() === '')).toEqual([]);
      }
    },
  );

  it('finds « 418 » and « teapot » in the status reference, which the home found nothing for', async () => {
    const source = await TOOLS.find((tool) => tool.id === 'http-status')!.reference!();
    const rows = source.rows((await source.words.fr()).default, (key) => key);
    const titles = (typed: string) =>
      rows
        .filter((row) => source.matches?.(row, typed) ?? matchesSearchable(row, typed))
        .map((row) => source.title(row));

    expect(titles('418')).toEqual(["418 I'm a teapot"]);
    expect(titles('TEAPOT')).toEqual(["418 I'm a teapot"]);
  });

  /** The rail leaves out the recents its section lists: enough are kept for five to remain. */
  it('keeps enough recent tools for the rail of the largest category to show five', () => {
    const largest = Math.max(
      ...TOOL_CATEGORIES.map((category) => TOOLS.filter((tool) => tool.category === category).length),
    );

    expect(KEPT_RECENT_TOOLS).toBeGreaterThanOrEqual(largest + MAX_RECENT_TOOLS);
  });

  /** What a sample fills reaches Rust as typing would: the tool asks with it. */
  it.each(TOOLS_THAT_ASK.map((tool) => [tool.id, tool] as const))(
    '%s offers a sample that the tool asks Rust with',
    async (_, tool) => {
      const harness = await renderTool<Tool>(await tool.load());
      expect(typeof harness.tool.sample).toBe('function');
      const before = JSON.stringify(harness.tools.asked);

      harness.tool.sample!();
      await harness.settle();

      await expect.poll(() => JSON.stringify(harness.tools.asked), { timeout: 2_000 }).not.toBe(before);
      TestBed.resetTestingModule();
    },
  );

  /** Read, never typed into: nothing to keep as a note and nothing to fill. */
  it.each(REFERENCES.map((tool) => [tool.id, tool] as const))(
    '%s offers neither a result nor a sample',
    async (_, tool) => {
      const harness = await renderTool<Tool>(await tool.load());

      expect(harness.tool.result).toBeUndefined();
      expect(harness.tool.sample).toBeUndefined();
      TestBed.resetTestingModule();
    },
  );
});
