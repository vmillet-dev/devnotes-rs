import { describe, expect, it } from 'vitest';
import { FakeToolsRepository } from '@testing/fake-tools-repository';
import { renderTool } from '@testing/tool-harness';
import { SlugToolComponent } from './slug-tool.component';

describe('SlugToolComponent', () => {
  const answer = (tools: FakeToolsRepository): void => {
    tools.slug = 'ete-2026';
  };

  it('asks for a lower-case slug on dashes by default, and shows it', async () => {
    const harness = await renderTool(SlugToolComponent, answer);

    await harness.type('slug-input', 'Été 2026 !', 'slugify');

    expect(harness.tools.requestsOf('slugify')).toEqual([
      { text: 'Été 2026 !', separator: 'dash', lowercase: true },
    ]);
    expect(harness.element('[data-testid="output-value"]').textContent).toBe('ete-2026');
  });

  it('asks again with the separator and the case chosen', async () => {
    const harness = await renderTool(SlugToolComponent, answer);
    await harness.type('slug-input', 'Été 2026', 'slugify');

    harness.element<HTMLButtonElement>('[data-segment-id="underscore"]').click();
    const lowercase = harness.element<HTMLInputElement>('[data-testid="slug-lowercase"]');
    lowercase.checked = false;
    lowercase.dispatchEvent(new Event('change'));
    await harness.type('slug-input', 'Été 2026', 'slugify');

    expect(harness.tools.requestsOf('slugify').at(-1)).toEqual({
      text: 'Été 2026',
      separator: 'underscore',
      lowercase: false,
    });
  });

  it('keeps the slug as a note', async () => {
    const harness = await renderTool(SlugToolComponent, answer);
    await harness.type('slug-input', 'Été 2026 !', 'slugify');

    expect(harness.tool.result()).toEqual({
      title: { key: 'tools.slug.noteTitle', params: { text: 'Été 2026 !' } },
      kind: 'snippet',
      language: 'txt',
      content: 'ete-2026',
    });
  });
});
