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

  /** « Structuré » did not say what it compares: the tab is « Données », with a line under it. */
  it('names the second mode « Données » and says under the tabs what it compares', async () => {
    const harness = await renderTool(DiffToolComponent);
    expect(mode(harness, 'structured').textContent?.trim()).toBe('Données');
    expect(harness.element('[data-testid="diff-data-line"]')).toBeNull();

    mode(harness, 'structured').click();
    await harness.settle();

    expect(harness.element('[data-testid="diff-data-line"]').textContent).toContain(
      'Compare les valeurs, pas le texte',
    );
  });

  it('goes back to the text mode from a side that is no structure, both texts kept', async () => {
    const harness = await renderTool(DiffToolComponent, (tools) => {
      tools.diffAnswer = { kind: 'notStructured', sides: ['a', 'b'] };
    });
    await harness.type('text-diff-a', 'server {', 'diff_text');
    await harness.type('text-diff-b', 'server { }', 'diff_text');
    mode(harness, 'structured').click();
    await harness.settle();
    await expect.poll(() => harness.element('[data-testid="structured-diff-as-text-a"]')).not.toBeNull();

    harness.element<HTMLButtonElement>('[data-testid="structured-diff-as-text-a"]').click();
    await harness.settle();

    expect(harness.element('[data-testid="structured-diff-a"]')).toBeNull();
    expect(harness.element<HTMLTextAreaElement>('[data-testid="text-diff-a"]').value).toBe('server {');
    expect(harness.element<HTMLTextAreaElement>('[data-testid="text-diff-b"]').value).toBe('server { }');
  });

  /** After « Exemple », each tab shows its own sample in both sides, never the other's text. */
  it('fills both sides with the sample of the tab on screen', async () => {
    const harness = await renderTool(DiffToolComponent);
    const value = (testid: string) => harness.element<HTMLTextAreaElement>(`[data-testid="${testid}"]`).value;

    harness.tool.sample();
    await harness.settle();
    expect(value('text-diff-a')).toContain('server {');
    expect(value('text-diff-b')).toContain('server {');

    mode(harness, 'structured').click();
    await harness.settle();
    harness.tool.sample();
    await harness.settle();
    expect(value('structured-diff-a')).toContain('name: devnotes');
    expect(value('structured-diff-b')).toContain('"name": "devnotes"');
    expect(value('structured-diff-a')).not.toContain('server {');
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
