import { describe, expect, it, vi } from 'vitest';
import { FakeToolsRepository } from '@testing/fake-tools-repository';
import { ToolHarness, renderTool } from '@testing/tool-harness';
import { JsonGeneratorToolComponent } from './json-generator-tool.component';

describe('JsonGeneratorToolComponent', () => {
  const answer = (tools: FakeToolsRepository): void => {
    tools.generated = {
      kind: 'generated',
      text: '[\n  { "id": 1 }\n]',
      seed: 4242,
      unsupported: [{ keyword: 'pattern', path: '$.properties.code' }],
    };
  };

  const requests = (harness: ToolHarness<JsonGeneratorToolComponent>) =>
    harness.tools.requestsOf('generate_json');

  it('draws three documents like the example, with a seed drawn for them', async () => {
    const harness = await renderTool(JsonGeneratorToolComponent, answer);

    await harness.type('json-generator-input', '{"id":1}', 'generate_json');

    expect(requests(harness)).toEqual([{ source: 'example', text: '{"id":1}', count: 3, seed: null }]);
    expect(harness.element('[data-testid="json-generator-output"]').textContent).toBe('[\n  { "id": 1 }\n]');
    expect(harness.element('[data-testid="json-generator-seed-used"]').textContent).toContain('4242');
  });

  it('keeps the seed used, so the same documents come back', async () => {
    const harness = await renderTool(JsonGeneratorToolComponent, answer);
    await harness.type('json-generator-input', '{"id":1}', 'generate_json');

    harness.element<HTMLButtonElement>('[data-testid="json-generator-keep-seed"]').click();
    await vi.waitFor(() => expect(requests(harness)).toHaveLength(2));

    expect(requests(harness)[1]).toMatchObject({ seed: 4242 });
    expect(harness.tool.actions()[0]!.disabled).toBe(true);
  });

  it('draws again without a seed, on request', async () => {
    const harness = await renderTool(JsonGeneratorToolComponent, answer);
    await harness.type('json-generator-input', '{"id":1}', 'generate_json');

    harness.tool.actions()[0]!.run();
    await vi.waitFor(() => expect(requests(harness)).toHaveLength(2));

    expect(requests(harness)[1]).toEqual(requests(harness)[0]);
  });

  it('lists what a schema asked that the generator did not honour', async () => {
    const harness = await renderTool(JsonGeneratorToolComponent, answer);
    harness
      .element<HTMLButtonElement>(
        '[data-testid="segmented-json-generator-source"] [data-segment-id="schema"]',
      )
      .click();

    await harness.type('json-generator-input', '{"type":"object"}', 'generate_json');

    expect(requests(harness).at(-1)).toMatchObject({ source: 'schema' });
    expect(harness.element('[data-testid="json-generator-unsupported"]').textContent).toContain(
      '$.properties.code',
    );
  });

  it('says where the source does not parse', async () => {
    const harness = await renderTool(JsonGeneratorToolComponent, (tools) => {
      tools.generated = { kind: 'unreadable', line: 2, column: 8 };
    });

    await harness.type('json-generator-input', '{\n  "a": ,\n}', 'generate_json');

    expect(harness.element('[data-testid="json-generator-unreadable"]').textContent).toContain('ligne 2');
    expect(harness.tool.result()).toBeNull();
  });
});
