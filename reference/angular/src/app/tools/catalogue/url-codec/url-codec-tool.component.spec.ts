import { describe, expect, it, vi } from 'vitest';
import { UrlParts } from '@core/model/tool-answers.model';
import { FakeToolsRepository } from '@testing/fake-tools-repository';
import { ToolHarness, renderTool } from '@testing/tool-harness';
import { UrlCodecToolComponent } from './url-codec-tool.component';

const PARTS: UrlParts = {
  scheme: 'https',
  username: 'ana',
  password: 'sécret',
  host: 'api.exemple.fr',
  port: 443,
  portIsDefault: true,
  path: '/v1/factures',
  query: 'statut=pay%C3%A9e',
  parameters: [{ name: 'statut', value: 'payée' }],
  fragment: null,
};

async function tab(harness: ToolHarness<UrlCodecToolComponent>, id: 'codec' | 'parse'): Promise<void> {
  harness.element<HTMLButtonElement>(`[data-testid="segmented-url-tab"] [data-segment-id="${id}"]`).click();
  await harness.settle();
}

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

  describe('the Analyser tab', () => {
    const parsed = (tools: FakeToolsRepository): void => {
      tools.url = { kind: 'parsed', parts: PARTS };
    };

    it('lists the parts Rust found, and the decoded parameters', async () => {
      const harness = await renderTool(UrlCodecToolComponent, parsed);
      await tab(harness, 'parse');

      await harness.type(
        'url-parser-input',
        'https://ana:s%C3%A9cret@api.exemple.fr/v1/factures',
        'parse_url',
      );

      expect(harness.all('[data-part]').map((part) => part.dataset['part'])).toEqual([
        'scheme',
        'username',
        'password',
        'host',
        'port',
        'path',
        'query',
      ]);
      expect(harness.element('[data-part="port"]').textContent).toContain('par défaut');
      expect(harness.element('[data-testid="url-parser-parameters"]').textContent).toContain('payée');
    });

    it('says a password is there without showing it, or copying it', async () => {
      const harness = await renderTool(UrlCodecToolComponent, parsed);
      await tab(harness, 'parse');
      await harness.type('url-parser-input', 'https://ana:s%C3%A9cret@api.exemple.fr/', 'parse_url');

      const password = harness.element('[data-part="password"]');
      expect(password.textContent).not.toContain('sécret');
      expect(password.querySelector('app-copy-value')).toBeNull();
    });

    it('keeps the parts as JSON, the password left behind', async () => {
      const harness = await renderTool(UrlCodecToolComponent, parsed);
      await tab(harness, 'parse');
      await harness.type('url-parser-input', 'https://api.exemple.fr/', 'parse_url');

      const result = harness.tool.result()!;
      expect(result.language).toBe('json');
      expect(result.title).toEqual({ key: 'tools.url-codec.noteParsed', params: { host: 'api.exemple.fr' } });
      expect(JSON.parse(result.content)).not.toHaveProperty('password');
      expect(result.content).not.toContain('sécret');
    });

    it('says why a URL is refused', async () => {
      const harness = await renderTool(UrlCodecToolComponent, (tools) => {
        tools.url = { kind: 'invalid', problem: 'noScheme' };
      });
      await tab(harness, 'parse');

      await harness.type('url-parser-input', 'exemple.fr/chemin', 'parse_url');

      expect(harness.element('[data-testid="url-parser-problem"]').textContent).toContain('protocole');
      expect(harness.tool.result()).toBeNull();
    });

    it('reads the text typed on the other tab, and gives it back', async () => {
      const harness = await renderTool(UrlCodecToolComponent, parsed);
      await harness.type('url-codec-decoded', 'https://api.exemple.fr/', 'url_codec');

      await tab(harness, 'parse');
      expect(harness.element<HTMLTextAreaElement>('[data-testid="url-parser-input"]').value).toBe(
        'https://api.exemple.fr/',
      );
      await harness.type('url-parser-input', 'https://api.exemple.fr/v2', 'parse_url');
      await tab(harness, 'codec');

      expect(harness.element<HTMLTextAreaElement>('[data-testid="url-codec-decoded"]').value).toBe(
        'https://api.exemple.fr/v2',
      );
    });
  });

  it('offers a decoded URL to the Analyser tab, and hands it over decoded', async () => {
    const harness = await renderTool(UrlCodecToolComponent, (tools) => {
      tools.urlCodecAnswer = { kind: 'done', text: 'https://api.exemple.fr/?q=café' };
      tools.url = { kind: 'parsed', parts: PARTS };
    });
    expect(harness.element('[data-testid="url-analyse-this"]')).toBeNull();

    await harness.type('url-codec-encoded', 'https%3A%2F%2Fapi.exemple.fr%2F%3Fq%3Dcaf%C3%A9', 'url_codec');
    await vi.waitFor(() =>
      expect(harness.tools.requestsOf('parse_url').at(-1)).toBe('https://api.exemple.fr/?q=café'),
    );
    await harness.settle();
    harness.element<HTMLButtonElement>('[data-testid="url-analyse-this"]').click();
    await harness.settle();

    expect(harness.element<HTMLTextAreaElement>('[data-testid="url-parser-input"]').value).toBe(
      'https://api.exemple.fr/?q=café',
    );
    expect(harness.element('[data-testid="url-parser-parts"]')).not.toBeNull();
  });

  it('offers nothing for a decoded text that is no URL', async () => {
    const harness = await renderTool(UrlCodecToolComponent, (tools) => {
      tools.urlCodecAnswer = { kind: 'done', text: 'caf%C3%A9' };
      tools.url = { kind: 'invalid', problem: 'noScheme' };
    });

    await harness.type('url-codec-decoded', 'café', 'url_codec');
    await vi.waitFor(() => expect(harness.tools.requestsOf('parse_url')).toEqual(['café']));
    await harness.settle();

    expect(harness.element('[data-testid="url-analyse-this"]')).toBeNull();
  });

  it('fills its sample encoded, so both tabs have something to show', async () => {
    const harness = await renderTool(UrlCodecToolComponent, encodes);

    harness.tool.sample();
    await harness.settle();

    expect(harness.element<HTMLTextAreaElement>('[data-testid="url-codec-encoded"]').value).toContain(
      '%C3%A9',
    );
  });
});
