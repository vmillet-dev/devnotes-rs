import { describe, expect, it } from 'vitest';
import { ToolHarness, renderTool } from '@testing/tool-harness';
import { MustacheSheetToolComponent } from './mustache-sheet-tool.component';
import { MUSTACHE_ENTRIES } from './mustache-sheet.data';
import en from './mustache-sheet.en.json';
import fr from './mustache-sheet.fr.json';

describe('MustacheSheetToolComponent', () => {
  type Harness = ToolHarness<MustacheSheetToolComponent>;

  async function render(): Promise<Harness> {
    const harness = await renderTool(MustacheSheetToolComponent);
    await expect
      .poll(() => harness.all('[data-testid="reference-row"]').length)
      .toBe(MUSTACHE_ENTRIES.length);
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
    ['{{#', ['{{#list}}…{{/list}}', '{{#flag}}…{{/flag}}', '{{#object}}…{{/object}}']],
    ['inclus', ['{{> partial}}']],
    ['inversée', ['{{^list}}…{{/list}}']],
    ['html', ['{{name}}', '{{{name}}}', '{{& name}}']],
  ])('finds %s', async (query, expected) => {
    const harness = await render();

    await search(harness, query);

    expect(keys(harness)).toEqual(expected);
  });

  it('shows a template, the view it is given, and what comes out — braces and all', async () => {
    const harness = await render();
    const example = row(harness, '{{name}}').querySelector('[data-testid="mustache-sheet-example"]')!;

    expect(example.querySelector('.template')?.textContent).toBe('Bonjour {{name}}');
    expect(example.querySelector('.view')?.textContent).toBe('{ "name": "Ada" }');
    expect(example.querySelector('[data-testid="mustache-sheet-output"]')?.textContent).toBe('Bonjour Ada');
  });

  it('renders a value the view lacks as nothing', async () => {
    const harness = await render();

    expect(
      row(harness, '{{missing}}').querySelector('[data-testid="mustache-sheet-output"]')?.textContent,
    ).toBe('[]');
  });

  it('empties its search on Vider', async () => {
    const harness = await render();
    await search(harness, 'section');

    harness.tool.clear();
    await harness.settle();

    expect(keys(harness)).toHaveLength(MUSTACHE_ENTRIES.length);
  });

  it('has the same words in both languages, for every tag', () => {
    const syntaxes = MUSTACHE_ENTRIES.map((entry) => entry.syntax).sort();

    expect(new Set(syntaxes).size).toBe(syntaxes.length);
    expect(Object.keys(fr.entries).sort()).toEqual(syntaxes);
    expect(Object.keys(en.entries).sort()).toEqual(syntaxes);
  });
});
