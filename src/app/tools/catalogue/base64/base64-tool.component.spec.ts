import { TestBed } from '@angular/core/testing';
import { describe, expect, it, vi } from 'vitest';
import { FILE_DIALOG_ADAPTER } from '@core/services/dialogs/file-dialog.service';
import { StatusNotifier } from '@core/services/notifications/status.service';
import { FakeFileDialog } from '@testing/fake-file-dialog';
import { ToolHarness, renderTool } from '@testing/tool-harness';
import { Base64ToolComponent } from './base64-tool.component';

describe('Base64ToolComponent', () => {
  const segment = (harness: ToolHarness<Base64ToolComponent>, kind: string, id: string) =>
    harness.element<HTMLButtonElement>(`[data-testid="segmented-${kind}"] [data-segment-id="${id}"]`);

  const text = (harness: ToolHarness<Base64ToolComponent>, testid: string) =>
    harness.element(`[data-testid="${testid}"]`)?.textContent?.trim();

  it('encodes a text with the standard alphabet, padded, by default', async () => {
    const harness = await renderTool(Base64ToolComponent, (tools) => {
      tools.base64 = 'w6l0w6k=';
    });

    await harness.type('base64-input', 'été', 'encode_base64');

    expect(harness.tools.requestsOf('encode_base64')).toEqual([
      { text: 'été', options: { alphabet: 'standard', padded: true } },
    ]);
    expect(text(harness, 'base64-output')).toBe('w6l0w6k=');
    expect(harness.tool.result()).toMatchObject({ content: 'w6l0w6k=', language: 'txt' });
  });

  it('encodes a file where it lies, by its path', async () => {
    const harness = await renderTool(Base64ToolComponent, (tools) => {
      tools.base64File = { kind: 'encoded', name: 'logo.png', bytes: 3, text: 'AP8Q' };
    });
    (TestBed.inject(FILE_DIALOG_ADAPTER) as FakeFileDialog).openPath = 'C:/images/logo.png';

    segment(harness, 'base64-source', 'file').click();
    await harness.settle();
    harness.element<HTMLButtonElement>('[data-testid="base64-pick-file"]').click();
    await vi.waitFor(() => expect(harness.tools.requestsOf('encode_base64_file')).toHaveLength(1));
    await harness.settle();

    expect(harness.tools.requestsOf('encode_base64_file')[0]).toMatchObject({ path: 'C:/images/logo.png' });
    expect(text(harness, 'base64-file')).toBe('logo.png · 3 octets');
    expect(text(harness, 'base64-output')).toBe('AP8Q');
  });

  it('decodes into text, and names the character a decoding stops at', async () => {
    const harness = await renderTool(Base64ToolComponent, (tools) => {
      tools.base64Decoded = { kind: 'invalid', problem: 'character', at: 6, character: '#' };
    });
    segment(harness, 'base64-direction', 'decode').click();

    await harness.type('base64-input', 'QUJD\nR#==', 'decode_base64');

    expect(text(harness, 'base64-problem')).toContain('« # »');
    expect(text(harness, 'base64-problem')).toContain('caractère 7');
    expect(harness.tool.result()).toBeNull();
  });

  it('offers to write bytes that are no text to a file, never to a note', async () => {
    const harness = await renderTool(Base64ToolComponent, (tools) => {
      tools.base64Decoded = { kind: 'binary', bytes: 3, preview: '00 ff 10' };
      tools.base64Saved = { kind: 'saved', bytes: 3 };
    });
    (TestBed.inject(FILE_DIALOG_ADAPTER) as FakeFileDialog).savePath = 'C:/out/decoded.bin';
    segment(harness, 'base64-direction', 'decode').click();
    await harness.type('base64-input', 'AP8Q', 'decode_base64');

    expect(harness.tool.result()).toBeNull();
    harness.element<HTMLButtonElement>('[data-testid="base64-save-file"]').click();
    await vi.waitFor(() => expect(harness.tools.requestsOf('save_base64')).toHaveLength(1));

    expect(harness.tools.requestsOf('save_base64')[0]).toMatchObject({
      text: 'AP8Q',
      path: 'C:/out/decoded.bin',
    });
    await vi.waitFor(() => expect(TestBed.inject(StatusNotifier).status()?.key).toBe('tools.base64.saved'));
  });
});
