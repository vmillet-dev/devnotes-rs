import { describe, expect, it } from 'vitest';
import { ToolHarness, renderTool } from '@testing/tool-harness';
import { MarkdownSheetToolComponent } from './markdown-sheet-tool.component';
import { MARKDOWN_ENTRIES } from './markdown-sheet.data';
import en from './markdown-sheet.en.json';
import fr from './markdown-sheet.fr.json';

describe('MarkdownSheetToolComponent', () => {
  type Harness = ToolHarness<MarkdownSheetToolComponent>;

  async function render(): Promise<Harness> {
    const harness = await renderTool(MarkdownSheetToolComponent);
    await expect
      .poll(() => harness.all('[data-testid="reference-row"]').length)
      .toBe(MARKDOWN_ENTRIES.length);
    return harness;
  }

  const keys = (harness: Harness) =>
    harness.all('[data-testid="reference-row"]').map((row) => row.dataset['key']);
  const row = (harness: Harness, syntax: string) =>
    harness.all('[data-testid="reference-row"]').find((tr) => tr.dataset['key'] === syntax)!;

  async function search(harness: Harness, query: string): Promise<void> {
    const field = harness.element<HTMLInputElement>('[data-testid="reference-search"]');
    field.value = query;
    field.dispatchEvent(new Event('input'));
    await harness.settle();
  }

  it.each([
    ['~~', ['~~…~~', '```lang']],
    ['alignement', ['| :-- | :-: | --: |']],
    ['note de bas de page', ['[^1]']],
    ['![', ['![…](url)']],
  ])('finds %s', async (query, expected) => {
    const harness = await render();

    await search(harness, query);

    expect(keys(harness)).toEqual(expected);
  });

  it('says where each construct works, CommonMark or GitHub', async () => {
    const harness = await render();
    const works = (syntax: string, flavour: string) =>
      row(harness, syntax).querySelector<HTMLElement>(`[data-testid="markdown-sheet-${flavour}"]`)!.dataset[
        'works'
      ];

    expect([works('**…**', 'commonmark'), works('**…**', 'github')]).toEqual(['true', 'true']);
    expect([works('| … |', 'commonmark'), works('| … |', 'github')]).toEqual(['false', 'true']);
    expect(row(harness, '- [ ] …').textContent).toContain('Non pris en charge');
  });

  it('shows the example as it is typed, line breaks included', async () => {
    const harness = await render();

    expect(row(harness, '- …').querySelector('.source')?.textContent).toBe('- un\n- deux');
  });

  it('copies spaces, not the symbols that draw them', async () => {
    const harness = await render();

    row(harness, '…␣␣').querySelector<HTMLButtonElement>('[data-testid="copy-value"]')!.click();
    await expect.poll(() => harness.clipboard.content).toBe('…  ');

    row(harness, '**…**').querySelector<HTMLButtonElement>('[data-testid="copy-value"]')!.click();
    await expect.poll(() => harness.clipboard.content).toBe('**…**');
  });

  it('empties its search on Vider', async () => {
    const harness = await render();
    await search(harness, 'lien');

    harness.tool.clear();
    await harness.settle();

    expect(keys(harness)).toHaveLength(MARKDOWN_ENTRIES.length);
  });

  it('has the same words in both languages, for every entry', () => {
    const syntaxes = MARKDOWN_ENTRIES.map((entry) => entry.syntax).sort();

    expect(new Set(syntaxes).size).toBe(syntaxes.length);
    expect(Object.keys(fr.entries).sort()).toEqual(syntaxes);
    expect(Object.keys(en.entries).sort()).toEqual(syntaxes);
  });
});
