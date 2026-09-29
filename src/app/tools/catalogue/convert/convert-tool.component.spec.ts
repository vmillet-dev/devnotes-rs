import { describe, expect, it } from 'vitest';
import { FakeToolsRepository } from '@testing/fake-tools-repository';
import { ToolHarness, renderTool } from '@testing/tool-harness';
import { ConvertToolComponent } from './convert-tool.component';

describe('ConvertToolComponent', () => {
  const answer = (tools: FakeToolsRepository): void => {
    tools.conversion = { kind: 'converted', text: 'service: billing\n' };
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

    harness.tool.actions()[0]!.run();
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

  it('says what the other format cannot hold, and where, with its convention', async () => {
    const harness = await renderTool(ConvertToolComponent, (tools) => {
      tools.conversion = { kind: 'impossible', crossing: 'tomlNull', path: '$.database.replica' };
    });
    harness
      .element<HTMLButtonElement>('[data-testid="segmented-convert-to"] [data-segment-id="toml"]')
      .click();

    await harness.type('convert-input', '{"database":{"replica":null}}', 'convert_data');

    expect(text(harness, 'convert-impossible')).toBe('TOML n’a pas de null : $.database.replica');
    expect(harness.element('[data-testid="convert-convention-toml"]')).not.toBeNull();
    expect(harness.element('[data-testid="convert-convention-xml"]')).toBeNull();
  });
});
