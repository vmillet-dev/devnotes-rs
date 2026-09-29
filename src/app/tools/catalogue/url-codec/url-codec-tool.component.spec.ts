import { describe, expect, it } from 'vitest';
import { FakeToolsRepository } from '@testing/fake-tools-repository';
import { renderTool } from '@testing/tool-harness';
import { UrlCodecToolComponent } from './url-codec-tool.component';

describe('UrlCodecToolComponent', () => {
  const encodes = (tools: FakeToolsRepository): void => {
    tools.urlCodecAnswer = { kind: 'done', text: 'caf%C3%A9' };
  };

  const value = (harness: { element: (s: string) => HTMLTextAreaElement }, testid: string) =>
    harness.element(`[data-testid="${testid}"]`).value;

  it('encodes what is typed on the left, as a component by default', async () => {
    const harness = await renderTool(UrlCodecToolComponent, encodes);

    await harness.type('url-codec-decoded', 'café', 'url_codec');

    expect(harness.tools.requestsOf('url_codec')).toEqual([
      { text: 'café', direction: 'encode', scope: 'component' },
    ]);
    expect(value(harness, 'url-codec-encoded')).toBe('caf%C3%A9');
    expect(harness.tool.result()).toMatchObject({
      content: 'caf%C3%A9',
      title: { key: 'tools.url-codec.noteEncoded' },
    });
  });

  it('decodes what is typed on the right, keeping it as it is', async () => {
    const harness = await renderTool(UrlCodecToolComponent, (tools) => {
      tools.urlCodecAnswer = { kind: 'done', text: 'café' };
    });

    await harness.type('url-codec-encoded', 'caf%C3%A9', 'url_codec');

    expect(harness.tools.requestsOf('url_codec').at(-1)).toMatchObject({ direction: 'decode' });
    expect(value(harness, 'url-codec-encoded')).toBe('caf%C3%A9');
    expect(value(harness, 'url-codec-decoded')).toBe('café');
  });

  it('never shows the last translation on the side now being typed from', async () => {
    const harness = await renderTool(UrlCodecToolComponent, encodes);
    await harness.type('url-codec-decoded', 'café', 'url_codec');

    const encoded = harness.element<HTMLTextAreaElement>('[data-testid="url-codec-encoded"]');
    encoded.value = 'th%C3%A9';
    encoded.dispatchEvent(new Event('input'));
    await harness.settle();

    expect(value(harness, 'url-codec-decoded')).toBe('');
    expect(harness.tool.result()).toBeNull();
  });

  it('says where an escape breaks, counting from one', async () => {
    const harness = await renderTool(UrlCodecToolComponent, (tools) => {
      tools.urlCodecAnswer = { kind: 'malformedEscape', at: 2 };
    });

    await harness.type('url-codec-encoded', 'ab%2G', 'url_codec');

    expect(harness.element('[data-testid="url-codec-problem"]').textContent).toContain('caractère 3');
    expect(harness.tool.result()).toBeNull();
  });

  it('asks again for a whole URL', async () => {
    const harness = await renderTool(UrlCodecToolComponent, encodes);
    await harness.type('url-codec-decoded', 'https://a.fr/b c', 'url_codec');

    harness
      .element<HTMLButtonElement>('[data-testid="segmented-url-codec-scope"] [data-segment-id="whole"]')
      .click();
    await harness.type('url-codec-decoded', 'https://a.fr/b c', 'url_codec');

    expect(harness.tools.requestsOf('url_codec').at(-1)).toMatchObject({ scope: 'whole' });
  });
});
