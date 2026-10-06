import { describe, expect, it } from 'vitest';
import { FakeToolsRepository } from '@testing/fake-tools-repository';
import { ToolHarness, renderTool } from '@testing/tool-harness';
import { ConvertToolComponent } from './convert-tool.component';

describe('ConvertToolComponent', () => {
  const answer = (tools: FakeToolsRepository): void => {
    tools.conversion = { kind: 'converted', text: 'service: billing\n', dropped: [] };
  };

  const text = (harness: ToolHarness<ConvertToolComponent>, testid: string): string =>
    harness.element(`[data-testid="${testid}"]`)?.textContent?.replace(/\s+/g, ' ').trim() ?? '';

  it('converts JSON to YAML by default, and keeps the result in its language', async () => {
    const harness = await renderTool(ConvertToolComponent, answer);

    await harness.type('convert-input', '{"service":"billing"}', 'convert_data');

    expect(harness.tools.requestsOf('convert_data')).toEqual([
      { text: '{"service":"billing"}', from: 'json', to: 'yaml' },
    ]);
    expect(harness.element('[data-testid="convert-output"]').textContent).toBe('service: billing\n');
    expect(harness.tool.result()).toMatchObject({
      title: { key: 'tools.convert.noteTitle', params: { from: 'JSON', to: 'YAML' } },
      language: 'yml',
      content: 'service: billing\n',
    });
  });

  it('keeps the result in the format it was converted to, until the next one lands', async () => {
    const harness = await renderTool(ConvertToolComponent, answer);
    await harness.type('convert-input', '{"service":"billing"}', 'convert_data');

    harness
      .element<HTMLButtonElement>('[data-testid="segmented-convert-to"] [data-segment-id="toml"]')
      .click();
    await harness.settle();

    expect(harness.tool.result()).toMatchObject({
      title: { params: { to: 'YAML' } },
      language: 'yml',
    });
  });

  it('swaps the formats, the result becoming what is converted back', async () => {
    const harness = await renderTool(ConvertToolComponent, answer);
    await harness.type('convert-input', '{"service":"billing"}', 'convert_data');

    harness.element<HTMLButtonElement>('[data-testid="convert-swap"]').click();
    await harness.settle();

    expect(harness.element<HTMLTextAreaElement>('[data-testid="convert-input"]').value).toBe(
      'service: billing\n',
    );
    await harness.type('convert-input', 'service: billing\n', 'convert_data');
    expect(harness.tools.requestsOf('convert_data').at(-1)).toMatchObject({ from: 'yaml', to: 'json' });
  });

  it('says where a text does not parse', async () => {
    const harness = await renderTool(ConvertToolComponent, (tools) => {
      tools.conversion = { kind: 'unreadable', line: 2, column: 10 };
    });

    await harness.type('convert-input', '{\n  "a": 1,,\n}', 'convert_data');

    expect(text(harness, 'convert-unreadable')).toBe('Le JSON ne se lit pas : ligne 2, colonne 10.');
    expect(harness.tool.result()).toBeNull();
  });

  /** A format is never converted to itself: the one the other side holds is disabled. */
  it('disables on each side the format the other holds', async () => {
    const harness = await renderTool(ConvertToolComponent, answer);
    const segment = (side: string, id: string) =>
      harness.element<HTMLButtonElement>(
        `[data-testid="segmented-convert-${side}"] [data-segment-id="${id}"]`,
      );

    expect(segment('from', 'yaml').getAttribute('aria-disabled')).toBe('true');
    expect(segment('to', 'json').getAttribute('aria-disabled')).toBe('true');
    segment('to', 'json').click();
    await harness.type('convert-input', '{"a":1}', 'convert_data');

    expect(harness.tools.requestsOf('convert_data').at(-1)).toMatchObject({ from: 'json', to: 'yaml' });
  });

  it('names the keys TOML left out, the first three and « … »', async () => {
    const harness = await renderTool(ConvertToolComponent, (tools) => {
      tools.conversion = {
        kind: 'converted',
        text: 'nom = "Dupont"\n',
        dropped: ['$.telephone', '$.fax', '$.adresse.etage', '$.mobile'],
      };
    });

    await harness.type('convert-input', '{"nom":"Dupont","telephone":null}', 'convert_data');

    expect(text(harness, 'convert-dropped')).toBe(
      '4 clés ignorées : $.telephone, $.fax, $.adresse.etage, …. TOML n’a pas de valeur null.',
    );
  });

  it('says what the other format cannot hold, and where, with its convention', async () => {
    const harness = await renderTool(ConvertToolComponent, (tools) => {
      tools.conversion = { kind: 'impossible', crossing: 'tomlNull', path: '$.replicas[1]' };
    });
    harness
      .element<HTMLButtonElement>('[data-testid="segmented-convert-to"] [data-segment-id="toml"]')
      .click();

    await harness.type('convert-input', '{"replicas":[1,null]}', 'convert_data');

    expect(text(harness, 'convert-impossible')).toBe(
      'TOML n’a pas de null, et une liste ne peut pas en perdre un sans décaler les autres : $.replicas[1]',
    );
    expect(harness.element('[data-testid="convert-convention-toml"]')).not.toBeNull();
    expect(harness.element('[data-testid="convert-convention-xml"]')).toBeNull();
  });
});
