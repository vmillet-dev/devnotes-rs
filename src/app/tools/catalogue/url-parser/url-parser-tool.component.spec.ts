import { describe, expect, it } from 'vitest';
import { UrlParts } from '@core/model/tool-answers.model';
import { FakeToolsRepository } from '@testing/fake-tools-repository';
import { renderTool } from '@testing/tool-harness';
import { UrlParserToolComponent } from './url-parser-tool.component';

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

describe('UrlParserToolComponent', () => {
  const parsed = (tools: FakeToolsRepository): void => {
    tools.url = { kind: 'parsed', parts: PARTS };
  };

  it('lists the parts Rust found, and the decoded parameters', async () => {
    const harness = await renderTool(UrlParserToolComponent, parsed);

    await harness.type('url-parser-input', 'https://ana:s%C3%A9cret@api.exemple.fr/v1/factures', 'parse_url');

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
    const harness = await renderTool(UrlParserToolComponent, parsed);
    await harness.type('url-parser-input', 'https://ana:s%C3%A9cret@api.exemple.fr/', 'parse_url');

    const password = harness.element('[data-part="password"]');
    expect(password.textContent).not.toContain('sécret');
    expect(password.querySelector('app-copy-value')).toBeNull();
  });

  it('keeps the parts as JSON, the password left behind', async () => {
    const harness = await renderTool(UrlParserToolComponent, parsed);
    await harness.type('url-parser-input', 'https://api.exemple.fr/', 'parse_url');

    const result = harness.tool.result()!;
    expect(result.language).toBe('json');
    expect(result.title).toEqual({ key: 'tools.url-parser.noteTitle', params: { host: 'api.exemple.fr' } });
    expect(JSON.parse(result.content)).not.toHaveProperty('password');
    expect(result.content).not.toContain('sécret');
  });

  it('says why a URL is refused', async () => {
    const harness = await renderTool(UrlParserToolComponent, (tools) => {
      tools.url = { kind: 'invalid', problem: 'noScheme' };
    });

    await harness.type('url-parser-input', 'exemple.fr/chemin', 'parse_url');

    expect(harness.element('[data-testid="url-parser-problem"]').textContent).toContain('protocole');
    expect(harness.tool.result()).toBeNull();
  });
});
