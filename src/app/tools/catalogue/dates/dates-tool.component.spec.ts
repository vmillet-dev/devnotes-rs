import { describe, expect, it, vi } from 'vitest';
import { InstantAnswer, InstantForms } from '@core/model/tool-answers.model';
import { FakeToolsRepository } from '@testing/fake-tools-repository';
import { ToolHarness, renderTool } from '@testing/tool-harness';
import { DatesToolComponent } from './dates-tool.component';

const FORMS: InstantForms = {
  unixSeconds: '1790000000',
  unixMilliseconds: '1790000000000',
  unixMicroseconds: '1790000000000000',
  unixNanoseconds: '1790000000000000000',
  isoUtc: '2026-09-21T14:13:20Z',
  isoLocal: '2026-09-21T16:13:20+02:00',
  localOffset: '+02:00',
  rfc2822: 'Mon, 21 Sep 2026 16:13:20 +0200',
  weekday: 1,
  weekDate: '2026-W39-1',
  week: 39,
  ordinalDate: '2026-264',
  dayOfYear: 264,
  epochMilliseconds: Date.now() - 3 * 24 * 3_600_000 - 60_000,
};

const READ: InstantAnswer = {
  kind: 'read',
  readAs: 'unix',
  magnitude: 'seconds',
  guessed: true,
  localAssumed: false,
  ambiguous: false,
  forms: FORMS,
};

describe('DatesToolComponent', () => {
  const answering =
    (instant: InstantAnswer) =>
    (tools: FakeToolsRepository): void => {
      tools.instant = instant;
    };

  const values = (harness: ToolHarness<DatesToolComponent>) =>
    harness.all('[data-testid="output-value"]').map((value) => value.textContent);

  it('asks nothing of an empty field and says what it will show', async () => {
    const harness = await renderTool(DatesToolComponent);

    expect(harness.tools.requestsOf('describe_instant')).toEqual([]);
    expect(harness.element('.empty')).not.toBeNull();
    expect(harness.tool.result()).toBeNull();
  });

  it('shows every form of a timestamp in three groups, and how it read it', async () => {
    const harness = await renderTool(DatesToolComponent, answering(READ));

    await harness.type('dates-input', '1790000000', 'describe_instant');

    expect(harness.tools.requestsOf('describe_instant')).toEqual([{ text: '1790000000', magnitude: null }]);
    expect(values(harness)).toEqual([
      '1790000000',
      '1790000000000',
      '1790000000000000',
      '1790000000000000000',
      '2026-09-21T14:13:20Z',
      '2026-09-21T16:13:20+02:00',
      'Mon, 21 Sep 2026 16:13:20 +0200',
      'il y a 3 jours',
      'lundi',
      '2026-W39-1',
      '39',
      '2026-264',
      '264',
    ]);
    expect(harness.all('[data-testid="dates-group"]').map((group) => group.dataset['group'])).toEqual([
      'unix',
      'formats',
      'landmarks',
    ]);
    expect(harness.element('[data-form="relative"] [data-testid="copy-value"]')).toBeNull();
    expect(harness.element('[data-form="weekDate"] [data-testid="copy-value"]')).not.toBeNull();
    expect(harness.all('[data-testid="copy-value"]')).toHaveLength(12);
    expect(harness.element('[data-testid="dates-reading"]').textContent).toContain(
      'timestamp Unix en secondes',
    );
    expect(harness.element('[data-testid="dates-reading"]').textContent).toContain('d’après sa taille');
  });

  it('titles the card with the local date in words, and the offset it is read at', async () => {
    const harness = await renderTool(
      DatesToolComponent,
      answering({ ...READ, forms: { ...FORMS, epochMilliseconds: Date.UTC(1971, 10, 8, 4, 25, 55) } }),
    );

    await harness.type('dates-input', '58422355', 'describe_instant');

    expect(harness.element('[data-testid="dates-heading"]').textContent?.trim()).toMatch(
      /^[A-Z][a-z]+ \d{1,2} novembre 1971, \d{2}:\d{2}:55$/,
    );
    expect(harness.element('.card-zone').textContent?.trim()).toBe('heure locale · UTC+02:00');
  });

  it('titles a date past what the page can hold with its ISO form', async () => {
    const harness = await renderTool(
      DatesToolComponent,
      answering({ ...READ, forms: { ...FORMS, epochMilliseconds: null } }),
    );

    await harness.type('dates-input', '2026-09-21T16:13:20+02:00', 'describe_instant');

    expect(harness.element('[data-testid="dates-heading"]').textContent?.trim()).toBe(
      '2026-09-21T16:13:20+02:00',
    );
  });

  it('forces a unit, and lets the guess come back', async () => {
    const harness = await renderTool(DatesToolComponent, answering(READ));
    await harness.type('dates-input', '1790000000', 'describe_instant');

    harness
      .element<HTMLButtonElement>(
        '[data-testid="segmented-dates-magnitude"] [data-segment-id="milliseconds"]',
      )
      .click();
    await vi.waitFor(() => expect(harness.tools.requestsOf('describe_instant')).toHaveLength(2));
    harness
      .element<HTMLButtonElement>('[data-testid="segmented-dates-magnitude"] [data-segment-id="auto"]')
      .click();
    await vi.waitFor(() => expect(harness.tools.requestsOf('describe_instant')).toHaveLength(3));

    expect(harness.tools.requestsOf('describe_instant').slice(1)).toEqual([
      { text: '1790000000', magnitude: 'milliseconds' },
      { text: '1790000000', magnitude: null },
    ]);
  });

  it('offers no unit for a date, and says when it took the local zone', async () => {
    const harness = await renderTool(
      DatesToolComponent,
      answering({ ...READ, readAs: 'weekDate', magnitude: null, guessed: false, localAssumed: true }),
    );

    await harness.type('dates-input', '2026-W39-1', 'describe_instant');

    expect(harness.element('[data-testid="segmented-dates-magnitude"]')).toBeNull();
    expect(harness.element('[data-testid="dates-reading"]').getAttribute('data-reading')).toBe('weekDate');
    expect(harness.element('[data-testid="dates-reading"] .guessed')).toBeNull();
    expect(harness.element('[data-testid="dates-local-assumed"]')).not.toBeNull();
    expect(harness.element('[data-testid="dates-ambiguous"]')).toBeNull();
  });

  it('says a repeated hour was read as the first, and leaves out forms that do not exist', async () => {
    const harness = await renderTool(
      DatesToolComponent,
      answering({
        ...READ,
        readAs: 'iso8601',
        ambiguous: true,
        forms: { ...FORMS, unixNanoseconds: null, rfc2822: null, epochMilliseconds: null },
      }),
    );

    await harness.type('dates-input', '2026-10-25T02:30', 'describe_instant');

    expect(harness.element('[data-testid="dates-ambiguous"]')).not.toBeNull();
    expect(harness.element('[data-form="unixNanoseconds"]')).toBeNull();
    expect(harness.element('[data-form="rfc2822"]')).toBeNull();
    expect(harness.element('[data-form="relative"]')).toBeNull();
    expect(harness.tool.result()?.content).not.toContain('RFC 2822');
  });

  it('says where a text stops being a date', async () => {
    const harness = await renderTool(DatesToolComponent, answering({ kind: 'unreadable', at: 6 }));

    await harness.type('dates-input', '2026-13-01', 'describe_instant');

    expect(harness.element('[data-testid="dates-problem"]').textContent).toContain('caractère 6');
    expect(harness.element('[data-testid="dates-reading"]')).toBeNull();
    expect(harness.tool.result()).toBeNull();
  });

  it('says a skipped hour and a date out of range in words', async () => {
    const harness = await renderTool(DatesToolComponent, answering({ kind: 'skipped' }));
    await harness.type('dates-input', '2026-03-29T02:30', 'describe_instant');
    expect(harness.element('[data-testid="dates-problem"]').getAttribute('data-problem')).toBe('skipped');

    harness.tools.instant = { kind: 'outOfRange' };
    await harness.type('dates-input', '1'.padEnd(40, '0'), 'describe_instant');

    expect(harness.element('[data-testid="dates-problem"]').getAttribute('data-problem')).toBe('outOfRange');
  });

  it('takes the present instant from Rust, in the field and in every form', async () => {
    const harness = await renderTool(DatesToolComponent, answering(READ));

    harness.element<HTMLButtonElement>('[data-testid="dates-now"]').click();
    await vi.waitFor(() => expect(harness.tools.requestsOf('describe_instant')).toHaveLength(1));
    await harness.settle();

    expect(harness.element<HTMLInputElement>('[data-testid="dates-input"]').value).toBe(
      '2026-09-29T12:03:12.000Z',
    );
    expect(harness.tools.requestsOf('describe_instant')[0]).toEqual({
      text: '2026-09-29T12:03:12.000Z',
      magnitude: null,
    });
  });

  it('leaves the field alone when the present instant cannot be had', async () => {
    const harness = await renderTool(DatesToolComponent);
    vi.spyOn(harness.tools, 'currentInstant').mockRejectedValue(new Error('no clock'));

    harness.element<HTMLButtonElement>('[data-testid="dates-now"]').click();
    await harness.settle();

    expect(harness.element<HTMLInputElement>('[data-testid="dates-input"]').value).toBe('');
  });

  it('keeps the instant as a note in forms any language reads', async () => {
    const harness = await renderTool(DatesToolComponent, answering(READ));

    await harness.type('dates-input', '1790000000', 'describe_instant');

    expect(harness.tool.result()).toEqual({
      title: { key: 'tools.dates.noteTitle', params: { instant: '2026-09-21T14:13:20Z' } },
      kind: 'snippet',
      language: 'txt',
      content: [
        'Unix: 1790000000',
        'Unix ms: 1790000000000',
        'ISO 8601: 2026-09-21T14:13:20Z',
        'ISO 8601: 2026-09-21T16:13:20+02:00',
        'RFC 2822: Mon, 21 Sep 2026 16:13:20 +0200',
      ].join('\n'),
    });
  });

  it('empties the field and the forced unit on Vider', async () => {
    const harness = await renderTool(DatesToolComponent, answering(READ));
    await harness.type('dates-input', '1790000000', 'describe_instant');
    harness
      .element<HTMLButtonElement>('[data-testid="segmented-dates-magnitude"] [data-segment-id="nanoseconds"]')
      .click();
    await vi.waitFor(() => expect(harness.tools.requestsOf('describe_instant')).toHaveLength(2));

    harness.tool.clear();
    await harness.settle();

    expect(harness.element<HTMLInputElement>('[data-testid="dates-input"]').value).toBe('');
    expect(harness.all('[data-testid="output-row"]')).toHaveLength(0);
    expect(harness.element('[data-testid="segmented-dates-magnitude"]')).toBeNull();
  });
});
