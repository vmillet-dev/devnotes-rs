import { describe, expect, it, vi } from 'vitest';
import { FakeToolsRepository } from '@testing/fake-tools-repository';
import { ToolHarness, renderTool } from '@testing/tool-harness';
import { LoremToolComponent } from './lorem-tool.component';

describe('LoremToolComponent', () => {
  const answer = (tools: FakeToolsRepository): void => {
    tools.loremAnswer = { text: 'Lorem ipsum dolor.\n\nSed do eiusmod.', seed: 9 };
  };

  const requests = (harness: ToolHarness<LoremToolComponent>) => harness.tools.requestsOf('lorem_ipsum');

  async function drawn(harness: ToolHarness<LoremToolComponent>, count: number): Promise<void> {
    await vi.waitFor(() => expect(requests(harness)).toHaveLength(count));
    await harness.settle();
  }

  it('opens on three paragraphs that start with the classic', async () => {
    const harness = await renderTool(LoremToolComponent, answer);
    await drawn(harness, 1);

    expect(requests(harness)[0]).toEqual({ unit: 'paragraphs', count: 3, opening: true, seed: null });
    expect(harness.all('[data-testid="lorem-output"] p').map((paragraph) => paragraph.textContent)).toEqual([
      'Lorem ipsum dolor.',
      'Sed do eiusmod.',
    ]);
  });

  it('asks again for the unit and the count chosen, and on request', async () => {
    const harness = await renderTool(LoremToolComponent, answer);
    await drawn(harness, 1);

    harness
      .element<HTMLButtonElement>('[data-testid="segmented-lorem-unit"] [data-segment-id="words"]')
      .click();
    await drawn(harness, 2);
    harness.tool.actions()[0]!.run();
    await drawn(harness, 3);

    expect(requests(harness).slice(1)).toEqual([
      { unit: 'words', count: 3, opening: true, seed: null },
      { unit: 'words', count: 3, opening: true, seed: null },
    ]);
  });

  it('keeps the text as a note', async () => {
    const harness = await renderTool(LoremToolComponent, answer);
    await drawn(harness, 1);

    expect(harness.tool.result()).toMatchObject({
      language: 'txt',
      content: 'Lorem ipsum dolor.\n\nSed do eiusmod.',
    });
  });
});
