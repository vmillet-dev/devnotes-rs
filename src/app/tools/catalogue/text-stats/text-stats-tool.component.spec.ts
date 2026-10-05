import { describe, expect, it, vi } from 'vitest';
import { TextStats } from '@core/model/tool-answers.model';
import { FakeToolsRepository } from '@testing/fake-tools-repository';
import { ToolHarness, renderTool } from '@testing/tool-harness';
import { TextStatsToolComponent } from './text-stats-tool.component';

const STATS: TextStats = {
  characters: 7,
  nonWhitespace: 5,
  words: 2,
  lines: 1,
  nonEmptyLines: 1,
  paragraphs: 1,
  codePoints: 8,
  utf16Units: 8,
  utf8Bytes: 10,
  distinct: 3,
  frequencies: [
    { character: 'e', codePoints: 'U+0065', invisible: null, count: 4, share: 57.142857 },
    { character: ' ', codePoints: 'U+00A0', invisible: 'noBreakSpace', count: 2, share: 28.57 },
    { character: 'é', codePoints: 'U+0065 U+0301', invisible: null, count: 1, share: null },
  ],
  frequenciesTruncated: false,
};

describe('TextStatsToolComponent', () => {
  const answering =
    (stats: TextStats) =>
    (tools: FakeToolsRepository): void => {
      tools.stats = stats;
    };

  const asked = (harness: ToolHarness<TextStatsToolComponent>) => harness.tools.requestsOf('text_stats');
  const value = (harness: ToolHarness<TextStatsToolComponent>, count: string) =>
    harness.element(`[data-count="${count}"] [data-testid="text-stats-value"]`)?.textContent;

  it('asks nothing of an empty text', async () => {
    const harness = await renderTool(TextStatsToolComponent);

    expect(asked(harness)).toEqual([]);
    expect(harness.tool.result()).toBeNull();
  });

  it('counts as a reader and as a program does, figures to read and not to copy', async () => {
    const harness = await renderTool(TextStatsToolComponent, answering(STATS));

    await harness.type('text-stats-input', 'eee e é', 'text_stats');

    expect(asked(harness)).toEqual([{ text: 'eee e é', foldCase: false, countWhitespace: false }]);
    expect(value(harness, 'characters')).toBe('7');
    expect(value(harness, 'utf16Units')).toBe('8');
    expect(harness.element('[data-count="characters"]').textContent).toContain('comme on les voit');
    expect(harness.all('[data-testid="text-stats-count"]')).toHaveLength(9);
    expect(harness.all('[data-testid="text-stats-count"] [data-testid="copy-value"]')).toEqual([]);
  });

  it('lists the characters most frequent first, the invisible ones named', async () => {
    const harness = await renderTool(TextStatsToolComponent, answering(STATS));

    await harness.type('text-stats-input', 'eee', 'text_stats');

    const rows = harness.all('[data-testid="text-stats-frequency"]');
    const cells = (row: HTMLElement) =>
      [...row.querySelectorAll('td')].map((cell) => cell.textContent?.trim()).join(' | ');
    expect(rows.map(cells)).toEqual([
      'e | U+0065 | 4 | 57.1 %',
      'espace insécable | U+00A0 | 2 | 28.6 %',
      'é | U+0065 U+0301 | 1 | 0.0 %',
    ]);
    expect(rows[1]?.dataset['invisible']).toBe('noBreakSpace');
    expect(harness.element<HTMLElement>('[data-testid="text-stats-frequency"] .bar').style.width).toBe(
      '100%',
    );
    expect(harness.element('[data-testid="text-stats-distinct"]').textContent?.trim()).toBe(
      '3 caractères distincts',
    );
    expect(harness.element('[data-testid="text-stats-truncated"]')).toBeNull();
  });

  it('folds the case and counts whitespace once asked', async () => {
    const harness = await renderTool(
      TextStatsToolComponent,
      answering({ ...STATS, frequenciesTruncated: true }),
    );
    await harness.type('text-stats-input', 'Abba', 'text_stats');

    for (const testid of ['text-stats-fold', 'text-stats-whitespace']) {
      const box = harness.element<HTMLInputElement>(`[data-testid="${testid}"]`);
      box.checked = true;
      box.dispatchEvent(new Event('change'));
    }
    await vi.waitFor(() =>
      expect(asked(harness).at(-1)).toEqual({ text: 'Abba', foldCase: true, countWhitespace: true }),
    );
    await harness.settle();

    expect(harness.element('[data-testid="text-stats-truncated"]').textContent).toContain('3');
  });

  it('keeps the counts as a note, named in the language on screen', async () => {
    const harness = await renderTool(TextStatsToolComponent, answering(STATS));

    await harness.type('text-stats-input', 'eee', 'text_stats');

    const result = harness.tool.result();
    expect(result?.title).toEqual({ key: 'tools.text-stats.noteTitle' });
    expect(result?.content.split('\n')[0]).toBe(
      `${'Caractères'.padEnd('Caractères hors espaces'.length)}  7`,
    );
    expect(result?.content.split('\n')).toHaveLength(9);
  });

  it('empties the text on Vider', async () => {
    const harness = await renderTool(TextStatsToolComponent, answering(STATS));
    await harness.type('text-stats-input', 'eee', 'text_stats');

    harness.tool.clear();
    await harness.settle();

    expect(harness.element<HTMLTextAreaElement>('[data-testid="text-stats-input"]').value).toBe('');
    expect(harness.all('[data-testid="text-stats-count"]')).toHaveLength(0);
  });
});
