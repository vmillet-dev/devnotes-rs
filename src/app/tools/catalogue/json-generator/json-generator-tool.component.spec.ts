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

  /**
   * « La garder » stood under the result, on the right, and the field it fills on the left
   * under the source: the seed has one place, its field, and a pin beside it.
   */
  it('says the seed just drawn at its own field, and pins it there', async () => {
    const harness = await renderTool(JsonGeneratorToolComponent, answer);
    const pin = () => harness.element<HTMLButtonElement>('[data-testid="json-generator-keep-seed"]');
    expect(pin()).toBeNull();
    await harness.type('json-generator-input', '{"id":1}', 'generate_json');

    const option = harness.element('[data-testid="json-generator-seed"]').closest('.seed-option')!;
    expect(option.querySelector('[data-testid="json-generator-seed-used"]')!.textContent).toBe(
      '· 4242 utilisée',
    );
    expect(option.contains(pin())).toBe(true);
    expect(pin().getAttribute('aria-pressed')).toBe('false');
    expect(harness.element('[data-testid="json-generator-seed-pinned"]')).toBeNull();

    pin().click();
    await vi.waitFor(() => expect(requests(harness)).toHaveLength(2));
    await harness.settle();

    expect(requests(harness)[1]).toMatchObject({ seed: 4242 });
    expect(harness.element<HTMLInputElement>('[data-testid="json-generator-seed"]').value).toBe('4242');
    expect(pin().getAttribute('aria-pressed')).toBe('true');
    expect(harness.element('[data-testid="json-generator-seed-used"]')).toBeNull();
    // Pinned, drawing again is still offered — and said to give the same documents.
    expect(harness.tool.actions()[0]).toMatchObject({
      labelKey: 'tools.json-generator.again',
      disabled: false,
    });
    expect(harness.element('[data-testid="json-generator-seed-pinned"]').textContent).toContain(
      '« Régénérer » redonne les mêmes documents',
    );

    // The pin lets it go again.
    pin().click();
    await vi.waitFor(() => expect(requests(harness)).toHaveLength(3));
    expect(requests(harness)[2]).toMatchObject({ seed: null });
  });

  /**
   * A sample document under « Schéma » answered three strings of lorem ipsum and a warning
   * about `createdAt · $`, `email · $`…
   */
  it('says an example is one rather than drawing from it as a schema, and reads it as a model on request', async () => {
    const harness = await renderTool(JsonGeneratorToolComponent, (tools) => {
      tools.generated = { kind: 'anExample' };
    });
    const source = (id: string) =>
      harness.element<HTMLButtonElement>(
        `[data-testid="segmented-json-generator-source"] [data-segment-id="${id}"]`,
      );
    source('schema').click();
    await harness.type('json-generator-input', '{"id":1,"email":"a@b.c"}', 'generate_json');

    expect(harness.element('[data-testid="json-generator-an-example"]').textContent).toContain(
      'Ceci ressemble à un exemple de document, pas à un schéma.',
    );
    expect(harness.element('[data-testid="json-generator-output"]')).toBeNull();
    expect(harness.tool.result()).toBeNull();
    // A text is there: nothing asks for one to be pasted.
    expect(harness.element('[data-testid="json-generator-empty"]')).toBeNull();

    harness.element<HTMLButtonElement>('[data-testid="json-generator-as-example"]').click();
    await vi.waitFor(() => expect(requests(harness).at(-1)).toMatchObject({ source: 'example' }));
    await harness.settle();

    expect(harness.element('[data-testid="json-generator-expects"]').dataset['source']).toBe('example');
    expect(harness.element<HTMLTextAreaElement>('[data-testid="json-generator-input"]').value).toBe(
      '{"id":1,"email":"a@b.c"}',
    );
  });

  /** Two things were called « Exemple » on this screen: the mode is « Modèle ». */
  it('says under the switch what each mode expects, and names the second « Modèle »', async () => {
    const harness = await renderTool(JsonGeneratorToolComponent, answer);
    const expects = () => harness.element('[data-testid="json-generator-expects"]');
    const source = (id: string) =>
      harness.element<HTMLButtonElement>(
        `[data-testid="segmented-json-generator-source"] [data-segment-id="${id}"]`,
      );

    expect(source('example').textContent?.trim()).toBe('Modèle');
    expect(expects().textContent).toContain('Modèle : un document modèle');

    source('schema').click();
    await harness.settle();
    expect(expects().dataset['source']).toBe('schema');
    expect(expects().textContent).toContain('Schéma : un JSON Schema décrit les documents.');
  });

  /** « Exemple » loaded a document whatever the switch said, a schema never. */
  it('fills the sample the mode on screen expects, and leaves the mode where it is', async () => {
    const harness = await renderTool(JsonGeneratorToolComponent, answer);
    const input = () => harness.element<HTMLTextAreaElement>('[data-testid="json-generator-input"]').value;

    harness.tool.sample();
    await harness.settle();
    expect(input()).toContain('"name": "Ada Lovelace"');
    await vi.waitFor(() => expect(requests(harness).at(-1)).toMatchObject({ source: 'example' }));

    harness
      .element<HTMLButtonElement>(
        '[data-testid="segmented-json-generator-source"] [data-segment-id="schema"]',
      )
      .click();
    harness.tool.sample();
    await harness.settle();
    expect(input()).toContain('"type": "object"');
    expect(input()).toContain('"properties"');
    expect(input()).not.toContain('Ada Lovelace');
    await vi.waitFor(() => expect(requests(harness).at(-1)).toMatchObject({ source: 'schema' }));
    // What it fills is a schema: it parses, and names only what the generator draws from.
    expect(() => JSON.parse(input()) as unknown).not.toThrow();
  });

  it('takes a typed seed within 32 bits, and lets it go when the field is emptied', async () => {
    const harness = await renderTool(JsonGeneratorToolComponent, answer);
    await harness.type('json-generator-input', '{"id":1}', 'generate_json');

    await harness.type('json-generator-seed', '99', 'generate_json');
    await harness.type('json-generator-seed', '99999999999', 'generate_json');
    await harness.type('json-generator-seed', '-5', 'generate_json');
    await harness.type('json-generator-seed', '', 'generate_json');

    expect(requests(harness).map((request) => (request as { seed: number | null }).seed)).toEqual([
      null,
      99,
      4_294_967_295,
      0,
      null,
    ]);
  });

  it('asks for as many documents as typed', async () => {
    const harness = await renderTool(JsonGeneratorToolComponent, answer);
    await harness.type('json-generator-input', '{"id":1}', 'generate_json');

    await harness.type('json-generator-count', '7', 'generate_json');

    expect(requests(harness).at(-1)).toMatchObject({ count: 7 });
  });

  it('empties the source and lets the seed go on Vider', async () => {
    const harness = await renderTool(JsonGeneratorToolComponent, answer);
    await harness.type('json-generator-input', '{"id":1}', 'generate_json');
    harness.element<HTMLButtonElement>('[data-testid="json-generator-keep-seed"]').click();
    await vi.waitFor(() => expect(requests(harness)).toHaveLength(2));

    harness.tool.clear();
    await harness.settle();

    expect(harness.element('[data-testid="json-generator-output"]')).toBeNull();
    expect(harness.element<HTMLInputElement>('[data-testid="json-generator-seed"]').value).toBe('');
    expect(harness.element('[data-testid="json-generator-seed-pinned"]')).toBeNull();
    expect(harness.element('[data-testid="json-generator-keep-seed"]')).toBeNull();
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
    const unsupported = harness.element('[data-testid="json-generator-unsupported"]');
    expect(unsupported.textContent).toContain('Mots-clés de schéma non pris en charge, ignorés :');
    // The keyword in code, then its place in words: nothing after « · » that reads as a price.
    expect(
      harness
        .element('[data-testid="json-generator-unsupported-item"]')
        .textContent?.replace(/\s+/g, ' ')
        .trim(),
    ).toBe('pattern (à $.properties.code)');
    expect(unsupported.textContent).not.toContain('·');
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
