import { describe, expect, it } from 'vitest';
import { ToolHarness, renderTool } from '@testing/tool-harness';
import { MimeTypesToolComponent } from './mime-types-tool.component';
import { MIME_GROUPS, MIME_TYPES, mimeGroup } from './mime-types.data';
import en from './mime-types.en.json';
import fr from './mime-types.fr.json';

describe('MimeTypesToolComponent', () => {
  type Harness = ToolHarness<MimeTypesToolComponent>;

  async function render(): Promise<Harness> {
    const harness = await renderTool(MimeTypesToolComponent);
    await expect.poll(() => harness.all('[data-testid="reference-row"]').length).toBe(MIME_TYPES.length);
    return harness;
  }

  const keys = (harness: Harness) =>
    harness.all('[data-testid="reference-row"]').map((row) => row.dataset['key']);

  async function search(harness: Harness, query: string): Promise<void> {
    const field = harness.element<HTMLInputElement>('[data-testid="reference-search"]');
    field.value = query;
    field.dispatchEvent(new Event('input'));
    await harness.settle();
  }

  it('groups the types by their top-level type, in a fixed order', async () => {
    const harness = await render();

    expect(harness.all('[data-testid="reference-group"]').map((group) => group.dataset['group'])).toEqual([
      ...MIME_GROUPS,
    ]);
    expect(harness.element('[data-group="font"] th')?.textContent).toContain('font/*');
  });

  it.each([
    ['.webp', ['image/webp']],
    ['webp', ['image/webp']],
    ['.json', ['application/json']],
    ['json', ['application/json']],
    ['image/png', ['image/png']],
  ])('finds %s', async (query, expected) => {
    const harness = await render();

    await search(harness, query);

    expect(keys(harness)).toEqual(expect.arrayContaining(expected));
  });

  it('finds .ts, and says it is not TypeScript', async () => {
    const harness = await render();

    await search(harness, '.ts');

    expect(keys(harness)).toContain('video/mp2t');
    expect(harness.element('[data-key="video/mp2t"]').textContent).toContain('TypeScript');
  });

  it('lists the extensions of a type, or says it has none', async () => {
    const harness = await render();

    expect(
      harness
        .all('[data-key="image/jpeg"] [data-testid="mime-types-extension"]')
        .map((tag) => tag.textContent),
    ).toEqual(['.jpg', '.jpeg', '.jfif']);
    expect(harness.element('[data-key="multipart/form-data"] .none')?.textContent?.trim()).toBe('aucune');
  });

  it('empties its search on Vider', async () => {
    const harness = await render();
    await search(harness, 'zip');

    harness.tool.clear();
    await harness.settle();

    expect(keys(harness)).toHaveLength(MIME_TYPES.length);
  });

  it('has a description in both languages for every type, each in a group it knows', () => {
    const types = MIME_TYPES.map((mime) => mime.type).sort();

    expect(new Set(types).size).toBe(types.length);
    expect(Object.keys(fr.types).sort()).toEqual(types);
    expect(Object.keys(en.types).sort()).toEqual(types);
    expect(MIME_TYPES.every((mime) => MIME_GROUPS.includes(mimeGroup(mime.type)))).toBe(true);
  });
});
