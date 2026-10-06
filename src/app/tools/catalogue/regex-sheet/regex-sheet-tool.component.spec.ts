import { describe, expect, it } from 'vitest';
import { ToolHarness, renderTool } from '@testing/tool-harness';
import { RegexSheetToolComponent } from './regex-sheet-tool.component';
import { REGEX_ENTRIES } from './regex-sheet.data';
import en from './regex-sheet.en.json';
import fr from './regex-sheet.fr.json';

/** The examples draw invisible characters as symbols. */
const visible = (text: string) => text.replaceAll('\n', '␊').replaceAll('\r', '␍').replaceAll('\t', '⇥');
const raw = (text: string) => text.replaceAll('␊', '\n').replaceAll('␍', '\r').replaceAll('⇥', '\t');

describe('RegexSheetToolComponent', () => {
  type Harness = ToolHarness<RegexSheetToolComponent>;

  async function render(): Promise<Harness> {
    const harness = await renderTool(RegexSheetToolComponent);
    await expect.poll(() => harness.all('[data-testid="reference-row"]').length).toBe(REGEX_ENTRIES.length);
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
    ['\\b', ['\\b']],
    ['\\B', ['\\B']],
    ['(?<', ['(?<name>…)', '(?<=…)', '(?<!…)']],
    ['lookbehind', ['(?<=…)', '(?<!…)']],
    ['paresseux', ['*?', 'U']],
    ['lazy', ['*?']],
  ])('finds %s', async (query, expected) => {
    const harness = await render();

    await search(harness, query);

    expect(keys(harness)).toEqual(expected);
  });

  it('says which flavour supports what, and on what condition', async () => {
    const harness = await render();
    const support = (syntax: string, flavour: string) =>
      row(harness, syntax).querySelector<HTMLElement>(`[data-testid="regex-sheet-${flavour}"]`)!;

    expect(support('(?>…)', 'js').dataset['support']).toBe('no');
    expect(support('(?>…)', 'pcre').dataset['support']).toBe('yes');
    expect(support('\\p{L}', 'js').textContent).toContain('avec u ou v');
    expect(support('(?<=…)', 'pcre').textContent).toContain('longueur bornée');
    expect(support('g', 'pcre').textContent).toContain('Non pris en charge');
  });

  it('shows an example and what it matches, or that it matches nothing', async () => {
    const harness = await render();
    const example = (syntax: string) =>
      row(harness, syntax).querySelector('[data-testid="regex-sheet-example"]')!;

    expect([...example('*?').querySelectorAll('mark')].map((mark) => mark.textContent)).toEqual([
      '<a>',
      '<b>',
    ]);
    expect(example('*+').textContent).toContain('aucune correspondance');
  });

  it('empties its search on Vider', async () => {
    const harness = await render();
    await search(harness, 'lazy');

    harness.tool.clear();
    await harness.settle();

    expect(keys(harness)).toHaveLength(REGEX_ENTRIES.length);
  });

  /** What a JavaScript example claims, this very engine does. */
  it.each(REGEX_ENTRIES.filter((entry) => entry.js !== 'no').map((entry) => [entry.syntax, entry] as const))(
    'matches what its example says in JavaScript: %s',
    (_, entry) => {
      const literal = /^\/(.*)\/([a-z]*)$/.exec(entry.example.pattern);
      const [source, flags] = literal ? [literal[1]!, literal[2]!] : [entry.example.pattern, ''];
      const unicode = entry.js === 'note' && !flags.includes('u') && !flags.includes('v') ? 'u' : '';
      const global = flags.includes('g') ? '' : 'g';
      const pattern = new RegExp(source, flags + unicode + global);

      const found = [...raw(entry.example.text).matchAll(pattern)].map((match) => visible(match[0]));

      expect(found).toEqual(entry.example.matches.map((match) => visible(raw(match))));
    },
  );

  it('has the same words in both languages, a condition for every « note »', () => {
    const syntaxes = REGEX_ENTRIES.map((entry) => entry.syntax).sort();

    expect(new Set(syntaxes).size).toBe(syntaxes.length);
    expect(Object.keys(fr.entries).sort()).toEqual(syntaxes);
    expect(Object.keys(en.entries).sort()).toEqual(syntaxes);
    for (const entry of REGEX_ENTRIES) {
      for (const words of [fr.entries, en.entries]) {
        const said = words[entry.syntax as keyof typeof words] as { js?: string; pcre?: string };
        expect(Boolean(said.js), `${entry.syntax} js`).toBe(entry.js === 'note');
        expect(Boolean(said.pcre), `${entry.syntax} pcre`).toBe(entry.pcre === 'note');
      }
    }
  });
});
