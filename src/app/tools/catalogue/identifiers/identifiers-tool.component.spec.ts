import { describe, expect, it, vi } from 'vitest';
import { IdInspection } from '@core/model/tool-answers.model';
import { FakeToolsRepository } from '@testing/fake-tools-repository';
import { ToolHarness, renderTool } from '@testing/tool-harness';
import { IdentifiersToolComponent } from './identifiers-tool.component';

const V7 = '01922b6e-4b30-7cc4-9a5c-6f2d8e1b3a77';

describe('IdentifiersToolComponent', () => {
  const answer = (tools: FakeToolsRepository): void => {
    tools.identifiers = [V7, '01922b6e-4b31-7000-8000-000000000001'];
    tools.inspection = {
      kind: 'uuid',
      version: 7,
      variant: 'rfc',
      created: '2024-09-25T23:05:01.488Z',
      nil: false,
    };
  };

  const inspecting =
    (inspection: IdInspection) =>
    (tools: FakeToolsRepository): void => {
      tools.inspection = inspection;
    };

  const generated = (harness: ToolHarness<IdentifiersToolComponent>) =>
    harness.tools.requestsOf('generate_identifiers');

  const DEFAULTS = { kind: 'uuidV4', count: 5, uppercase: false, nanoLength: 21, nanoAlphabet: 'urlSafe' };

  async function pick(harness: ToolHarness<IdentifiersToolComponent>, kind: string): Promise<void> {
    harness.element<HTMLButtonElement>('[data-testid="choice-identifiers-kind"]').click();
    await harness.settle();
    harness.element<HTMLButtonElement>(`[data-testid="choice-option"][data-option-id="${kind}"]`).click();
    await harness.settle();
  }

  it('draws v4 UUIDs at once, one per line', async () => {
    const harness = await renderTool(IdentifiersToolComponent, answer);
    await vi.waitFor(() => expect(generated(harness)).toHaveLength(1));
    await harness.settle();

    expect(generated(harness)[0]).toEqual(DEFAULTS);
    expect(harness.element('[data-testid="identifiers-list"]').textContent).toBe(
      `${V7}\n01922b6e-4b31-7000-8000-000000000001`,
    );
    expect(harness.tool.result()).toMatchObject({
      title: { key: 'tools.identifiers.noteTitle', params: { kind: 'UUID v4' } },
      content: `${V7}\n01922b6e-4b31-7000-8000-000000000001`,
    });
  });

  it('asks for the kind chosen, and draws again on request', async () => {
    const harness = await renderTool(IdentifiersToolComponent, answer);
    await vi.waitFor(() => expect(generated(harness)).toHaveLength(1));

    await pick(harness, 'ulid');
    await vi.waitFor(() => expect(generated(harness)).toHaveLength(2));
    harness.element<HTMLButtonElement>('[data-testid="identifiers-generate"]').click();
    await vi.waitFor(() => expect(generated(harness)).toHaveLength(3));

    expect(generated(harness).slice(1)).toEqual([
      { ...DEFAULTS, kind: 'ulid' },
      { ...DEFAULTS, kind: 'ulid' },
    ]);
    expect(harness.element('[data-testid="identifiers-hint"]').textContent).toContain('Crockford');
  });

  it('offers the case only where it carries nothing, and NanoID its length and alphabet', async () => {
    const harness = await renderTool(IdentifiersToolComponent, answer);
    await vi.waitFor(() => expect(generated(harness)).toHaveLength(1));
    expect(harness.element('[data-testid="identifiers-uppercase"]')).not.toBeNull();
    expect(harness.element('[data-testid="identifiers-nano"]')).toBeNull();

    await pick(harness, 'nanoId');
    expect(harness.element('[data-testid="identifiers-uppercase"]')).toBeNull();
    await harness.type('identifiers-nano-length', '10', 'generate_identifiers');
    harness
      .element<HTMLButtonElement>('[data-testid="segmented-identifiers-alphabet"] [data-segment-id="digits"]')
      .click();
    await vi.waitFor(() => expect(generated(harness).at(-1)).toMatchObject({ nanoAlphabet: 'digits' }));
    const length = harness.element<HTMLInputElement>('[data-testid="identifiers-nano-length"]');
    length.value = '';
    length.dispatchEvent(new Event('input'));
    await harness.settle();

    expect(generated(harness).at(-1)).toEqual({
      ...DEFAULTS,
      kind: 'nanoId',
      nanoLength: 10,
      nanoAlphabet: 'digits',
    });
  });

  it('asks for the count and the case as they are set', async () => {
    const harness = await renderTool(IdentifiersToolComponent, answer);
    await vi.waitFor(() => expect(generated(harness)).toHaveLength(1));

    await harness.type('identifiers-count', '12', 'generate_identifiers');
    const count = harness.element<HTMLInputElement>('[data-testid="identifiers-count"]');
    count.value = '';
    count.dispatchEvent(new Event('input'));
    const uppercase = harness.element<HTMLInputElement>('[data-testid="identifiers-uppercase"]');
    uppercase.checked = true;
    uppercase.dispatchEvent(new Event('change'));
    await vi.waitFor(() => expect(generated(harness)).toHaveLength(3));

    expect(generated(harness).at(-1)).toEqual({ ...DEFAULTS, count: 12, uppercase: true });
  });

  it('says a v7’s version, its variant and when it was made', async () => {
    const harness = await renderTool(IdentifiersToolComponent, answer);

    await harness.type('identifiers-checked', V7, 'inspect_identifier');

    expect(harness.element('[data-testid="identifiers-recognised"]').textContent?.trim()).toBe('UUID');
    expect(harness.element('[data-testid="identifiers-version"]').textContent?.trim()).toBe('v7');
    expect(harness.element('[data-testid="identifiers-facts"]').textContent).toContain('RFC 9562');
    expect(harness.element('[data-testid="identifiers-created"]').textContent).toBe(
      '2024-09-25 23:05:01.488 UTC',
    );
  });

  it('names a nil UUID, and gives no time for one that carries none', async () => {
    const harness = await renderTool(
      IdentifiersToolComponent,
      inspecting({ kind: 'uuid', version: 0, variant: 'ncs', created: null, nil: true }),
    );

    await harness.type('identifiers-checked', '00000000-0000-0000-0000-000000000000', 'inspect_identifier');

    expect(harness.element('[data-testid="identifiers-version"]').textContent?.trim()).toBe('UUID nul');
    expect(harness.element('[data-testid="identifiers-created"]')).toBeNull();
  });

  it('reads the time of a ULID, an ObjectId, a KSUID and a first CUID', async () => {
    const harness = await renderTool(
      IdentifiersToolComponent,
      inspecting({ kind: 'ulid', created: '2016-07-30T23:54:10.259Z' }),
    );
    const facts = () => harness.element('[data-testid="identifiers-facts"]');

    await harness.type('identifiers-checked', '01ARZ3NDEKTSV4RRFFQ69G5FAV', 'inspect_identifier');
    expect(facts().dataset['kind']).toBe('ulid');
    expect(harness.element('[data-testid="identifiers-created"]').textContent).toBe(
      '2016-07-30 23:54:10.259 UTC',
    );

    for (const inspection of [
      { kind: 'objectId', created: '2012-10-17T21:13:27.000Z', counter: 4427793 },
      { kind: 'ksuid', created: '2017-10-10T04:00:47.000Z', payload: 'B5A1CD34B5F99D1154FB6853345C9735' },
      { kind: 'cuidV1', created: '2012-09-13T23:03:12.926Z' },
    ] as IdInspection[]) {
      harness.tools.inspection = inspection;
      await harness.type('identifiers-checked', `${inspection.kind} id`, 'inspect_identifier');
      expect(facts().dataset['kind']).toBe(inspection.kind);
    }
    expect(facts().textContent).toContain('dépréciée');
  });

  it('says what carries nothing, what is out of range, and what is no identifier', async () => {
    const harness = await renderTool(
      IdentifiersToolComponent,
      inspecting({ kind: 'possible', kinds: ['cuid2', 'nanoId'] }),
    );

    await harness.type('identifiers-checked', 'tz4a98xxat96iws9zmbrgj3a', 'inspect_identifier');
    expect(harness.element('[data-testid="identifiers-possible"]').textContent).toContain('CUID2 / NanoID');

    harness.tools.inspection = { kind: 'outOfRange', id: 'ulid' };
    await harness.type('identifiers-checked', '8ZZZZZZZZZZZZZZZZZZZZZZZZZ', 'inspect_identifier');
    expect(harness.element('[data-testid="identifiers-out-of-range"]').textContent).toContain('ULID');

    harness.tools.inspection = { kind: 'unrecognised' };
    await harness.type('identifiers-checked', 'not an id', 'inspect_identifier');
    expect(harness.element('[data-testid="identifiers-unrecognised"]')).not.toBeNull();
  });

  it('empties the identifier being read on Vider, and keeps the drawn ones', async () => {
    const harness = await renderTool(IdentifiersToolComponent, answer);
    await harness.type('identifiers-checked', V7, 'inspect_identifier');

    harness.tool.clear();
    await harness.settle();

    expect(harness.element<HTMLInputElement>('[data-testid="identifiers-checked"]').value).toBe('');
    expect(harness.element('[data-testid="identifiers-facts"]')).toBeNull();
    expect(harness.element('[data-testid="identifiers-list"]').textContent).toContain(V7);
  });
});
