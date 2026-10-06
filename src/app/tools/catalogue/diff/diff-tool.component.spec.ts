import { describe, expect, it } from 'vitest';
import { ToolHarness, renderTool } from '@testing/tool-harness';
import { DiffToolComponent } from './diff-tool.component';

describe('DiffToolComponent', () => {
  const mode = (harness: ToolHarness<DiffToolComponent>, id: string) =>
    harness.element<HTMLButtonElement>(`[data-testid="segmented-diff-mode"] [data-segment-id="${id}"]`);

  it('opens on the text mode, and the structured one compares the same two texts', async () => {
    const harness = await renderTool(DiffToolComponent);
    expect(harness.element('[data-testid="text-diff-a"]')).not.toBeNull();

    await harness.type('text-diff-a', '{"a":1}', 'diff_text');
    mode(harness, 'structured').click();
    await harness.settle();

    expect(harness.element('[data-testid="text-diff-a"]')).toBeNull();
    expect(harness.element<HTMLTextAreaElement>('[data-testid="structured-diff-a"]').value).toBe('{"a":1}');
  });

  it('switches to the structured mode when asked to compare as values, never on its own', async () => {
    const harness = await renderTool(DiffToolComponent, (tools) => {
      tools.textDiff = { ...tools.textDiff, identical: false, bothJson: true };
    });
    await harness.type('text-diff-a', '{"a":1}', 'diff_text');
    expect(harness.element('[data-testid="structured-diff-a"]')).toBeNull();

    harness.element<HTMLButtonElement>('[data-testid="text-diff-as-values"]').click();
    await harness.settle();

    expect(harness.element('[data-testid="structured-diff-a"]')).not.toBeNull();
  });

  it('hands the frame the actions, the result and the sample of the mode on screen', async () => {
    const harness = await renderTool(DiffToolComponent);
    expect(harness.tool.actions().map((action) => action.id)).toEqual(['swap', 'copy-unified']);

    mode(harness, 'structured').click();
    await harness.settle();
    expect(harness.tool.actions().map((action) => action.id)).toEqual(['swap', 'copy-patch']);

    harness.tool.sample();
    await harness.settle();
    expect(harness.element<HTMLTextAreaElement>('[data-testid="structured-diff-a"]').value).toContain(
      'name: devnotes',
    );

    harness.tool.clear();
    await harness.settle();
    expect(harness.element<HTMLTextAreaElement>('[data-testid="structured-diff-a"]').value).toBe('');
    expect(harness.tool.result()).toBeNull();
  });
});
