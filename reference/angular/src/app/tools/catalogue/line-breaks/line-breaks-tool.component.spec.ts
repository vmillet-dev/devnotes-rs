import { describe, expect, it, vi } from 'vitest';
import { FakeToolsRepository } from '@testing/fake-tools-repository';
import { ToolHarness, renderTool } from '@testing/tool-harness';
import { LineBreaksToolComponent } from './line-breaks-tool.component';

describe('LineBreaksToolComponent', () => {
  const answer = (tools: FakeToolsRepository): void => {
    tools.lineBreaks = {
      found: { lf: 1, crlf: 1, cr: 0 },
      text: 'a\nb\n',
      converted: 1,
      trimmed: 1,
      finalNewline: 'unchanged',
    };
  };

  /** A real paste carries its CRs, where a textarea's value would have turned them into LFs. */
  async function paste(harness: ToolHarness<LineBreaksToolComponent>, text: string): Promise<void> {
    const before = harness.tools.requestsOf('fix_line_breaks').length;
    // jsdom has no `DataTransfer`: the event carries what a paste would.
    const event = new Event('paste', { bubbles: true, cancelable: true });
    Object.defineProperty(event, 'clipboardData', { value: { getData: () => text } });
    harness.element('[data-testid="line-breaks-input"]').dispatchEvent(event);
    await vi.waitFor(() =>
      expect(harness.tools.requestsOf('fix_line_breaks').length).toBeGreaterThan(before),
    );
    await harness.settle();
  }

  it('keeps a pasted text raw, and asks with the options as they stand', async () => {
    const harness = await renderTool(LineBreaksToolComponent, answer);

    await paste(harness, 'a  \r\nb\n');

    expect(harness.tools.requestsOf('fix_line_breaks')).toEqual([
      { text: 'a  \r\nb\n', ending: null, trimTrailing: true, finalNewline: 'keep' },
    ]);
  });

  it('draws the endings and the trailing spaces it was given', async () => {
    const harness = await renderTool(LineBreaksToolComponent, answer);

    await paste(harness, 'a  \r\nb\n');

    const input = harness.element('[data-testid="line-breaks-input"]');
    expect(input.querySelector('[data-ending="crlf"]')).not.toBeNull();
    expect(input.querySelector('.trailing')?.textContent).toBe('  ');
  });

  it('says so when a text mixes its endings, and counts what changed', async () => {
    const harness = await renderTool(LineBreaksToolComponent, answer);

    await paste(harness, 'a\r\nb\n');

    expect(harness.element('[data-testid="line-breaks-mixed"]')).not.toBeNull();
    expect(harness.element('[data-testid="line-breaks-changes"]').textContent).toContain(
      '1 fin de ligne convertie',
    );
  });

  it('asks again for the ending chosen', async () => {
    const harness = await renderTool(LineBreaksToolComponent, answer);
    await paste(harness, 'a\r\nb\n');

    harness
      .element<HTMLButtonElement>('[data-testid="segmented-line-breaks-ending"] [data-segment-id="lf"]')
      .click();
    await vi.waitFor(() => expect(harness.tools.requestsOf('fix_line_breaks')).toHaveLength(2));

    expect(harness.tools.requestsOf('fix_line_breaks').at(-1)).toMatchObject({ ending: 'lf' });
  });

  it('reads the clipboard itself on Coller, and keeps the fixed text as a note', async () => {
    const harness = await renderTool(LineBreaksToolComponent, answer);
    harness.clipboard.content = 'x\r\ny';

    harness.element<HTMLButtonElement>('[data-testid="line-breaks-paste"]').click();
    await vi.waitFor(() => expect(harness.tools.requestsOf('fix_line_breaks')).toHaveLength(1));
    await harness.settle();

    expect(harness.tools.requestsOf('fix_line_breaks')[0]).toMatchObject({ text: 'x\r\ny' });
    expect(harness.tool.result()).toMatchObject({ language: 'txt', content: 'a\nb\n' });
  });
});
