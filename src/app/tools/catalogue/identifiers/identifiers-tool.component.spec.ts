import { describe, expect, it, vi } from 'vitest';
import { IdInspection } from '@core/model/tool-answers.model';
import { FakeToolsRepository } from '@testing/fake-tools-repository';
import { ToolHarness, renderTool } from '@testing/tool-harness';
import { IdentifiersToolComponent } from './identifiers-tool.component';

const V7 = '01922b6e-4b30-7cc4-9a5c-6f2d8e1b3a77';
const SECOND = '01922b6e-4b31-7000-8000-000000000001';

describe('IdentifiersToolComponent', () => {
  const answer = (tools: FakeToolsRepository): void => {
    tools.identifiers = [V7, SECOND];
    tools.inspection = {
      kind: 'uuid',
      version: 7,
      variant: 'rfc',
      created: '2024-09-25T23:05:01.488Z',
      nil: false,
      random: { text: '7cc4-9a5c-6f2d8e1b3a77', bits: 74 },
    };
  };

  const inspecting =
    (inspection: IdInspection) =>
    (tools: FakeToolsRepository): void => {
      tools.inspection = inspection;
    };

  const generated = (harness: ToolHarness<IdentifiersToolComponent>) =>
    harness.tools.requestsOf('generate_identifiers');

  const values = (harness: ToolHarness<IdentifiersToolComponent>) =>
    harness.all('[data-testid="identifiers-value"]').map((value) => value.textContent);

  const DEFAULTS = {
    kind: 'uuidV4',
    count: 5,
    uppercase: false,
    hyphens: true,
    nanoLength: 21,
    nanoAlphabet: 'urlSafe',
  };

  async function pick(harness: ToolHarness<IdentifiersToolComponent>, kind: string): Promise<void> {
    harness
      .element<HTMLButtonElement>(`[data-testid="segmented-identifiers-kind"] [data-segment-id="${kind}"]`)
      .click();
    await harness.settle();
  }

  async function drawn(harness: ToolHarness<IdentifiersToolComponent>, times: number): Promise<void> {
    await vi.waitFor(() => expect(generated(harness)).toHaveLength(times));
    await harness.settle();
  }

  it('draws v4 UUIDs at once, one row each, counted in the card’s header', async () => {
    const harness = await renderTool(IdentifiersToolComponent, answer);
    await drawn(harness, 1);

    expect(generated(harness)[0]).toEqual(DEFAULTS);
    expect(values(harness)).toEqual([V7, SECOND]);
    expect(harness.element('[data-testid="identifiers-counted"]').textContent?.trim()).toBe('2 identifiants');
    expect(harness.tool.result()).toMatchObject({
      title: { key: 'tools.identifiers.noteTitle', params: { kind: 'UUID v4' } },
      content: `${V7}\n${SECOND}`,
    });
  });

  it('dims a UUID’s hyphens and picks out its version digit, with or without hyphens', async () => {
    const harness = await renderTool(IdentifiersToolComponent, (tools) => {
      tools.identifiers = [V7, '01922b6e4b307cc49a5c6f2d8e1b3a77'];
    });
    await drawn(harness, 1);

    const [hyphenated, simple] = harness.all('[data-testid="identifiers-value"]');
    expect([...hyphenated.querySelectorAll('.hyphen')]).toHaveLength(4);
    expect(hyphenated.querySelector('.version')?.textContent).toBe('7');
    expect(simple.querySelector('.hyphen')).toBeNull();
    expect(simple.querySelector('.version')?.textContent).toBe('7');
  });

  it('cuts no other kind', async () => {
    const harness = await renderTool(IdentifiersToolComponent, (tools) => {
      tools.identifiers = ['01ARZ3NDEKTSV4RRFFQ69G5FAV'];
    });
    await pick(harness, 'ulid');
    await drawn(harness, 2);

    expect(harness.element('[data-testid="identifiers-value"] .version')).toBeNull();
  });

  it('asks for the kind of the tab chosen, says what it is, and draws again on request', async () => {
    const harness = await renderTool(IdentifiersToolComponent, answer);
    await drawn(harness, 1);

    await pick(harness, 'ulid');
    await drawn(harness, 2);
    harness.element<HTMLButtonElement>('[data-testid="identifiers-generate"]').click();
    await drawn(harness, 3);

    expect(generated(harness).slice(1)).toEqual([
      { ...DEFAULTS, kind: 'ulid' },
      { ...DEFAULTS, kind: 'ulid' },
    ]);
    expect(harness.element('[data-testid="identifiers-hint"]').textContent?.trim()).toMatch(
      /^ULID : .*Crockford/,
    );
    expect(harness.all('[data-testid="segmented-identifiers-kind"] [data-segment-id]')).toHaveLength(7);
  });

  it('draws again on R, but never on an R typed in a field', async () => {
    const harness = await renderTool(IdentifiersToolComponent, answer);
    await drawn(harness, 1);
    const press = (target: EventTarget, key = 'r', init: KeyboardEventInit = {}) =>
      target.dispatchEvent(new KeyboardEvent('keydown', { key, bubbles: true, cancelable: true, ...init }));

    press(document.body);
    await drawn(harness, 2);
    press(harness.element('[data-testid="identifiers-checked"]'));
    press(document.body, 'r', { ctrlKey: true });
    press(document.body, 'x');
    await harness.settle();

    expect(generated(harness)).toHaveLength(2);
    expect(harness.element('[data-testid="identifiers-generate"]').getAttribute('aria-keyshortcuts')).toBe(
      'R',
    );
  });

  it('steps the count within its bounds', async () => {
    const harness = await renderTool(IdentifiersToolComponent, answer);
    await drawn(harness, 1);
    const fewer = harness.element<HTMLButtonElement>('[data-testid="identifiers-fewer"]');
    const more = harness.element<HTMLButtonElement>('[data-testid="identifiers-more"]');

    more.click();
    await vi.waitFor(() => expect(generated(harness).at(-1)).toMatchObject({ count: 6 }));
    for (let i = 0; i < 6; i += 1) fewer.click();
    await vi.waitFor(() => expect(generated(harness).at(-1)).toMatchObject({ count: 1 }));
    await harness.settle();

    expect(fewer.disabled).toBe(true);
    expect(harness.element<HTMLInputElement>('[data-testid="identifiers-count"]').value).toBe('1');
  });

  it('offers the case only where it carries nothing, the hyphens for a UUID, and NanoID its length and alphabet', async () => {
    const harness = await renderTool(IdentifiersToolComponent, answer);
    await drawn(harness, 1);
    expect(harness.element('[data-testid="identifiers-uppercase"]')).not.toBeNull();
    expect(harness.element('[data-testid="identifiers-without-hyphens"]')).not.toBeNull();
    expect(harness.element('[data-testid="identifiers-nano"]')).toBeNull();

    await pick(harness, 'nanoId');
    expect(harness.element('[data-testid="identifiers-uppercase"]')).toBeNull();
    expect(harness.element('[data-testid="identifiers-without-hyphens"]')).toBeNull();
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

  it('asks for the count, the case and the hyphens as they are set', async () => {
    const harness = await renderTool(IdentifiersToolComponent, answer);
    await drawn(harness, 1);

    await harness.type('identifiers-count', '12', 'generate_identifiers');
    const count = harness.element<HTMLInputElement>('[data-testid="identifiers-count"]');
    count.value = '';
    count.dispatchEvent(new Event('input'));
    harness.element<HTMLInputElement>('[data-testid="identifiers-uppercase"]').click();
    harness.element<HTMLInputElement>('[data-testid="identifiers-without-hyphens"]').click();
    await vi.waitFor(() =>
      expect(generated(harness).at(-1)).toEqual({ ...DEFAULTS, count: 12, uppercase: true, hyphens: false }),
    );
  });

  it('copies one identifier from its row, and tints the row while « Copié » shows', async () => {
    const harness = await renderTool(IdentifiersToolComponent, answer);
    await drawn(harness, 1);

    harness.all('[data-testid="identifiers-row"] [data-testid="copy-value"]')[1].click();
    await vi.waitFor(() => expect(harness.clipboard.content).toBe(SECOND));

    const rows = harness.all('[data-testid="identifiers-row"]');
    await vi.waitFor(() =>
      expect(rows.map((row) => row.classList.contains('copied'))).toEqual([false, true]),
    );
    expect(rows[1].textContent).toContain('Copié');
  });

  it('copies every identifier at once', async () => {
    const harness = await renderTool(IdentifiersToolComponent, answer);
    await drawn(harness, 1);

    harness.element('[data-testid="identifiers-list"] .card-head [data-testid="copy-value"]').click();

    await vi.waitFor(() => expect(harness.clipboard.content).toBe(`${V7}\n${SECOND}`));
    expect(harness.element('.card-head').textContent).toContain('Copier tout');
  });

  it('says a v7’s format, version, variant, when it was made and what of it was drawn', async () => {
    const harness = await renderTool(IdentifiersToolComponent, answer);

    await harness.type('identifiers-checked', ` ${V7} `, 'inspect_identifier');

    expect(harness.element('[data-testid="identifiers-recognised"]').textContent?.trim()).toBe('UUID');
    expect(harness.element('[data-testid="identifiers-facts"]').textContent).toContain(
      '36 caractères, hexadécimal',
    );
    expect(harness.element('[data-testid="identifiers-version"]').textContent?.trim()).toBe('v7');
    expect(harness.element('[data-testid="identifiers-facts"]').textContent).toContain('RFC 9562');
    expect(harness.element('[data-testid="identifiers-created"]').textContent).toBe(
      '2024-09-25 23:05:01.488 UTC',
    );
    expect(harness.element('[data-testid="identifiers-random"]').textContent?.trim()).toBe(
      '7cc4-9a5c-6f2d8e1b3a77 · 74 bits',
    );
  });

  it('names a nil UUID, and gives no time nor random part for one that carries none', async () => {
    const harness = await renderTool(
      IdentifiersToolComponent,
      inspecting({ kind: 'uuid', version: 0, variant: 'ncs', created: null, nil: true, random: null }),
    );

    await harness.type('identifiers-checked', '00000000-0000-0000-0000-000000000000', 'inspect_identifier');

    expect(harness.element('[data-testid="identifiers-version"]').textContent?.trim()).toBe('UUID nul');
    expect(harness.element('[data-testid="identifiers-created"]')).toBeNull();
    expect(harness.element('[data-testid="identifiers-random"]')).toBeNull();
  });

  it('reads the time of a ULID, an ObjectId, a KSUID and a first CUID', async () => {
    const harness = await renderTool(
      IdentifiersToolComponent,
      inspecting({
        kind: 'ulid',
        created: '2016-07-30T23:54:10.259Z',
        random: { text: 'TSV4RRFFQ69G5FAV', bits: 80 },
      }),
    );
    const facts = () => harness.element('[data-testid="identifiers-facts"]');

    await harness.type('identifiers-checked', '01ARZ3NDEKTSV4RRFFQ69G5FAV', 'inspect_identifier');
    expect(facts().dataset['kind']).toBe('ulid');
    expect(facts().textContent).toContain('26 caractères, base32');
    expect(harness.element('[data-testid="identifiers-created"]').textContent).toBe(
      '2016-07-30 23:54:10.259 UTC',
    );
    expect(harness.element('[data-testid="identifiers-random"]').textContent?.trim()).toBe(
      'TSV4RRFFQ69G5FAV · 80 bits',
    );

    for (const inspection of [
      {
        kind: 'objectId',
        created: '2012-10-17T21:13:27.000Z',
        counter: 4427793,
        random: { text: 'bcf86cd799', bits: 40 },
      },
      {
        kind: 'ksuid',
        created: '2017-10-10T04:00:47.000Z',
        random: { text: 'B5A1CD34B5F99D1154FB6853345C9735', bits: 128 },
      },
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
    expect(values(harness)).toContain(V7);
  });
});
