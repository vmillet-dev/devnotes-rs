import { describe, expect, it } from 'vitest';
import { TextDiffAnswer } from '@core/model/tool-answers.model';
import { FakeToolsRepository } from '@testing/fake-tools-repository';
import { ToolHarness, renderTool } from '@testing/tool-harness';
import { TextDiffComponent } from './text-diff.component';

const UNIFIED = '--- a\n+++ b\n@@ -1,2 +1,2 @@\n host = db\n-port = 5432\n+port = 6432\n';

const COMPARED: TextDiffAnswer = {
  added: 1,
  removed: 1,
  rows: [
    {
      kind: 'same',
      left: { number: 1, pieces: [{ text: 'host = db', changed: false }] },
      right: { number: 1, pieces: [{ text: 'host = db', changed: false }] },
    },
    {
      kind: 'modified',
      left: {
        number: 2,
        pieces: [
          { text: 'port = ', changed: false },
          { text: '5432', changed: true },
        ],
      },
      right: {
        number: 2,
        pieces: [
          { text: 'port = ', changed: false },
          { text: '6432', changed: true },
        ],
      },
    },
  ],
  rowsTruncated: false,
  unified: UNIFIED,
  identical: false,
  bothJson: false,
};

describe('TextDiffComponent', () => {
  async function compared(answer: TextDiffAnswer = COMPARED): Promise<ToolHarness<TextDiffComponent>> {
    const harness = await renderTool(TextDiffComponent, (tools: FakeToolsRepository) => {
      tools.textDiff = answer;
    });
    await harness.type('text-diff-a', 'host = db\nport = 5432', 'diff_text');
    await harness.type('text-diff-b', 'host = db\nport = 6432', 'diff_text');
    return harness;
  }

  const text = (harness: ToolHarness<TextDiffComponent>, testid: string) =>
    harness.element(`[data-testid="${testid}"]`)?.textContent?.replace(/\s+/g, ' ').trim() ?? '';

  const last = (harness: ToolHarness<TextDiffComponent>) => harness.tools.requestsOf('diff_text').at(-1);

  it('compares line by line, line endings ignored and nothing else, by default', async () => {
    const harness = await compared();

    expect(last(harness)).toEqual({
      a: 'host = db\nport = 5432',
      b: 'host = db\nport = 6432',
      granularity: 'lines',
      ignoreTrailingWhitespace: false,
      ignoreAllWhitespace: false,
      ignoreCase: false,
      ignoreLineEndings: true,
    });
    expect([text(harness, 'text-diff-added'), text(harness, 'text-diff-removed')]).toEqual([
      '+ 1 ligne',
      '− 1 ligne',
    ]);
  });

  it('asks again in the granularity and with the options chosen', async () => {
    const harness = await compared();

    harness
      .element<HTMLButtonElement>('[data-testid="segmented-text-diff-granularity"] [data-segment-id="words"]')
      .click();
    harness.element<HTMLInputElement>('[data-testid="text-diff-ignore-case"]').click();
    harness.element<HTMLInputElement>('[data-testid="text-diff-ignore-endings"]').click();
    await harness.settle();
    await expect
      .poll(() => last(harness))
      .toMatchObject({ granularity: 'words', ignoreCase: true, ignoreLineEndings: false });

    expect(text(harness, 'text-diff-added')).toBe('+ 1 mot');
  });

  it('marks what changed inside a modified line, on both sides', async () => {
    const harness = await compared();

    expect(harness.all('[data-testid="text-diff-row"]').map((row) => row.dataset['kind'])).toEqual([
      'same',
      'modified',
    ]);
    expect(harness.all('[data-testid="text-diff-changed"]').map((piece) => piece.textContent)).toEqual([
      '5432',
      '6432',
    ]);
  });

  it('shows the unified diff, copies it and keeps it as a text snippet', async () => {
    const harness = await compared();

    harness
      .element<HTMLButtonElement>('[data-testid="segmented-text-diff-layout"] [data-segment-id="unified"]')
      .click();
    await harness.settle();

    const lines = harness.all('[data-testid="text-diff-unified"] .unified-line');
    expect(lines.map((line) => [line.dataset['kind'], line.textContent])).toEqual([
      ['file', '--- a'],
      ['file', '+++ b'],
      ['hunk', '@@ -1,2 +1,2 @@'],
      ['same', ' host = db'],
      ['removed', '-port = 5432'],
      ['added', '+port = 6432'],
    ]);
    expect(harness.tool.result()).toMatchObject({ kind: 'snippet', language: 'txt', content: UNIFIED });

    harness.tool
      .actions()
      .find((action) => action.id === 'copy-unified')!
      .run();
    await expect.poll(() => harness.clipboard.content).toBe(UNIFIED);
  });

  it('says two texts are identical, with nothing to copy or keep', async () => {
    const harness = await compared({
      ...COMPARED,
      identical: true,
      unified: '',
      rows: [],
      added: 0,
      removed: 0,
    });

    expect(harness.element('[data-testid="text-diff-identical"]')).not.toBeNull();
    expect(harness.tool.result()).toBeNull();
    expect(harness.tool.actions().find((action) => action.id === 'copy-unified')!.disabled).toBe(true);
  });

  it('offers to compare two JSON texts as values, and leaves the choice to the user', async () => {
    const harness = await compared({ ...COMPARED, bothJson: true });
    let asked = 0;
    harness.tool.asValues.subscribe(() => asked++);

    expect(text(harness, 'text-diff-json')).toContain('Les deux textes sont du JSON.');
    harness.element<HTMLButtonElement>('[data-testid="text-diff-as-values"]').click();

    expect(asked).toBe(1);
  });

  it('swaps the two texts, and a sample fills both', async () => {
    const harness = await compared();

    harness.tool
      .actions()
      .find((action) => action.id === 'swap')!
      .run();
    await harness.settle();
    expect(harness.element<HTMLTextAreaElement>('[data-testid="text-diff-a"]').value).toBe(
      'host = db\nport = 6432',
    );

    harness.tool.sample();
    await harness.settle();
    expect(harness.element<HTMLTextAreaElement>('[data-testid="text-diff-a"]').value).toContain('listen 80;');
    expect(harness.element<HTMLTextAreaElement>('[data-testid="text-diff-b"]').value).toContain(
      'listen 443 ssl;',
    );

    harness.tool.clear();
    await harness.settle();
    expect(harness.element<HTMLTextAreaElement>('[data-testid="text-diff-b"]').value).toBe('');
  });
});
