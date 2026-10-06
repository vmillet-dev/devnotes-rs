import { describe, expect, it } from 'vitest';
import { ToolHarness, renderTool } from '@testing/tool-harness';
import { AsciiToolComponent, codeIn } from './ascii-tool.component';
import { ASCII_CODES, CONTROLS, asciiGroup, binary, ctrlKey, hex, octal } from './ascii.data';
import en from './ascii.en.json';
import fr from './ascii.fr.json';

describe('AsciiToolComponent', () => {
  type Harness = ToolHarness<AsciiToolComponent>;

  async function render(): Promise<Harness> {
    const harness = await renderTool(AsciiToolComponent);
    await expect.poll(() => harness.all('[data-testid="reference-row"]').length).toBe(128);
    return harness;
  }

  const keys = (harness: Harness) =>
    harness.all('[data-testid="reference-row"]').map((row) => row.dataset['key']);
  const cells = (harness: Harness, code: number) =>
    harness.all(`[data-key="${code}"] td`).map((cell) => cell.textContent?.replace(/\s+/g, ' ').trim());

  async function search(harness: Harness, query: string): Promise<void> {
    const field = harness.element<HTMLInputElement>('[data-testid="reference-search"]');
    field.value = query;
    field.dispatchEvent(new Event('input'));
    await harness.settle();
  }

  it('writes a character in every base, its name, its escape and its Ctrl key', async () => {
    const harness = await render();

    expect(cells(harness, 10).slice(0, 8)).toEqual([
      '10',
      '0A',
      '012',
      '00001010',
      'LF',
      'Line FeedSaut de ligne : la fin de ligne d’Unix.',
      '\\n',
      '^J',
    ]);
    expect(harness.element('[data-key="10"] .standard').textContent).toBe('Line Feed');
    expect(cells(harness, 65).slice(0, 5)).toEqual(['65', '41', '101', '01000001', 'A']);
    expect(cells(harness, 64)[5]).toBe('Arobase');
  });

  it('groups the characters as the standard lays them out', async () => {
    const harness = await render();

    expect(harness.all('[data-testid="reference-group"]').map((group) => group.dataset['group'])).toEqual([
      'control',
      'punctuation',
      'digits',
      'upper',
      'lower',
    ]);
    expect([0, 31, 127, 32, 47, 48, 57, 65, 90, 91, 97, 122, 126].map(asciiGroup)).toEqual([
      'control',
      'control',
      'control',
      'punctuation',
      'punctuation',
      'digits',
      'digits',
      'upper',
      'upper',
      'punctuation',
      'lower',
      'lower',
      'punctuation',
    ]);
  });

  it.each([
    ['A', ['65']],
    ['a', ['97']],
    ['6', ['6', '54']],
    ['65', ['65']],
    ['0x41', ['65']],
    ['1b', ['27']],
    ['\\x7f', ['127']],
    ['0o101', ['65']],
    ['0b1000001', ['65']],
    ['newline', ['10']],
    ['xoff', ['19']],
    ['^C', ['3']],
    ['\\t', ['9']],
  ])('finds %s', async (query, expected) => {
    const harness = await render();

    await search(harness, query);

    expect(keys(harness)).toEqual(expected);
  });

  it('finds words in the language on screen', async () => {
    const harness = await render();

    await search(harness, 'arobase');

    expect(keys(harness)).toEqual(['64']);
  });

  it('copies the character itself, named aloud by its abbreviation for a control', async () => {
    const harness = await render();

    harness.element<HTMLButtonElement>('[data-key="65"] [data-testid="copy-value"]').click();
    await expect.poll(() => harness.clipboard.content).toBe('A');

    harness.element<HTMLButtonElement>('[data-key="9"] [data-testid="copy-value"]').click();
    await expect.poll(() => harness.clipboard.content).toBe('\t');
    expect(harness.element('[data-key="9"] [data-testid="copy-value"]').getAttribute('aria-label')).toContain(
      'HT',
    );
    expect(
      harness.element('[data-key="32"] [data-testid="copy-value"]').getAttribute('aria-label'),
    ).toContain('Espace');
  });

  it('empties its search on Vider', async () => {
    const harness = await render();
    await search(harness, 'A');

    harness.tool.clear();
    await harness.settle();

    expect(keys(harness)).toHaveLength(128);
  });

  it('reads a code in each base, and nothing else', () => {
    expect(['65', '0x41', '0X41', '41h', 'ff', '0o101', '0b1000001', 'abc', '1234'].map(codeIn)).toEqual([
      65,
      65,
      65,
      null,
      255,
      65,
      65,
      null,
      null,
    ]);
    expect([hex(10), octal(10), binary(10), ctrlKey(0), ctrlKey(127), ctrlKey(65)]).toEqual([
      '0A',
      '012',
      '00001010',
      '^@',
      '^?',
      null,
    ]);
  });

  it('has the same words in both languages: every control, every symbol, no letter or digit', () => {
    const named = ASCII_CODES.filter(
      (code) => asciiGroup(code) === 'control' || asciiGroup(code) === 'punctuation',
    )
      .map(String)
      .sort();

    expect(Object.keys(fr.codes).sort()).toEqual(named);
    expect(Object.keys(en.codes).sort()).toEqual(named);
    expect(Object.keys(CONTROLS)).toHaveLength(33);
  });
});
