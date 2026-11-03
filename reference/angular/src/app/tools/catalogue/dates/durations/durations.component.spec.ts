import { describe, expect, it, vi } from 'vitest';
import { DurationsAnswer, GapAnswer } from '@core/model/tool-answers.model';
import { FakeToolsRepository } from '@testing/fake-tools-repository';
import { ToolHarness, renderTool } from '@testing/tool-harness';
import { DurationsComponent } from './durations.component';

const MEASURED: GapAnswer = {
  kind: 'measured',
  negative: false,
  calendar: { years: 0, months: 6, days: 22, hours: 7, minutes: 41, seconds: 0 },
  elapsed: {
    days: 206,
    hours: 6,
    minutes: 41,
    seconds: 0,
    weeks: 29,
    weekDays: 3,
    totalHours: 4950,
    totalMinutes: 297_041,
  },
  iso: 'P6M22DT7H41M',
  transition: { date: '2026-03-29', forward: true, minutes: 60, wallMinutes: 297_101 },
  ambiguous: [],
};

const ANSWER: DurationsAnswer = {
  zone: 'Europe/Paris',
  gap: MEASURED,
  duration: {
    kind: 'read',
    negative: false,
    parts: { years: 0, months: 0, weeks: 0, days: 0, hours: 1, minutes: 30, seconds: 0 },
    iso: 'PT1H30M',
    fromWords: false,
    totals: { seconds: 5400, minutes: 90, hours: 1.5 },
  },
};

describe('DurationsComponent', () => {
  const answering =
    (answer: DurationsAnswer) =>
    (tools: FakeToolsRepository): void => {
      tools.durations = answer;
    };

  const text = (harness: ToolHarness<DurationsComponent>, selector: string) =>
    harness.element(selector)?.textContent?.replace(/\s+/g, ' ').trim();

  const rowValue = (harness: ToolHarness<DurationsComponent>, attribute: string, id: string) =>
    text(harness, `[${attribute}="${id}"] [data-testid="output-value"]`);

  it('asks nothing while every field is empty', async () => {
    const harness = await renderTool(DurationsComponent);

    expect(harness.tools.requestsOf('measure_durations')).toEqual([]);
    expect(harness.tool.result()).toBeNull();
  });

  it('spells the calendar gap, and cuts the elapsed time several ways', async () => {
    const harness = await renderTool(DurationsComponent, answering(ANSWER));

    await harness.type('dates-from', '2026-03-12 09:00', 'measure_durations');
    await harness.type('dates-to', '2026-10-04 16:41', 'measure_durations');

    expect(harness.tools.requestsOf('measure_durations').at(-1)).toEqual({
      from: '2026-03-12 09:00',
      to: '2026-10-04 16:41',
      duration: '',
    });
    expect(text(harness, '[data-testid="dates-gap-calendar"]')).toBe('6 mois, 22 jours, 7 h 41 min');
    expect(rowValue(harness, 'data-elapsed', 'days')).toBe('206 j 6 h 41');
    expect(rowValue(harness, 'data-elapsed', 'weeks')).toBe('29 sem 3 j');
    expect(rowValue(harness, 'data-elapsed', 'hours')).toBe('4 950 h 41');
    expect(rowValue(harness, 'data-elapsed', 'minutes')).toBe('297 041');
    expect(rowValue(harness, 'data-elapsed', 'iso')).toBe('P6M22DT7H41M');
    expect(text(harness, '[data-testid="dates-zone"]')).toBe('Heures saisies en Europe/Paris.');
  });

  it('says by how much a change to summer time parts the two counts', async () => {
    const harness = await renderTool(DurationsComponent, answering(ANSWER));

    await harness.type('dates-from', '2026-03-12 09:00', 'measure_durations');

    expect(text(harness, '[data-testid="dates-transition"]')).toBe(
      'Le passage à l’heure d’été du 29 mars tombe entre les deux dates : l’écart calendaire ' +
        '(4 951 h 41) compte une heure de plus que le temps réellement écoulé (4 950 h 41).',
    );
  });

  it('says a change back to winter time, undated, by its minutes', async () => {
    const harness = await renderTool(
      DurationsComponent,
      answering({
        ...ANSWER,
        gap: { ...MEASURED, transition: { date: null, forward: false, minutes: -30, wallMinutes: 297_011 } },
      }),
    );

    await harness.type('dates-from', '2026-03-12 09:00', 'measure_durations');

    expect(text(harness, '[data-testid="dates-transition"]')).toContain('compte 30 min de moins');
    expect(text(harness, '[data-testid="dates-transition"]')).toMatch(/^Un changement d’heure/);
  });

  it('signs a gap counted backwards, and names a time that came twice', async () => {
    const harness = await renderTool(
      DurationsComponent,
      answering({
        ...ANSWER,
        gap: {
          ...MEASURED,
          negative: true,
          calendar: { years: 1, months: 0, days: 1, hours: 0, minutes: 0, seconds: 0 },
          transition: null,
          ambiguous: ['to'],
        },
      }),
    );

    await harness.type('dates-from', '2026-03-12', 'measure_durations');

    expect(text(harness, '[data-testid="dates-gap-calendar"]')).toBe('−1 an, 1 jour');
    expect(harness.element('[data-testid="dates-gap-negative"]')).not.toBeNull();
    expect(text(harness, '[data-testid="dates-gap-ambiguous"]')).toMatch(/^À : cette heure a lieu deux fois/);
    expect(harness.element('[data-testid="dates-transition"]')).toBeNull();
  });

  it('says which field cannot be read, and where', async () => {
    const harness = await renderTool(
      DurationsComponent,
      answering({ ...ANSWER, gap: { kind: 'unreadable', field: 'to', at: 6 } }),
    );

    await harness.type('dates-to', '2026-13-01', 'measure_durations');

    expect(text(harness, '[data-testid="dates-gap-problem"]')).toBe('À : illisible à partir du caractère 6.');

    harness.tools.durations = { ...ANSWER, gap: { kind: 'skipped', field: 'from' } };
    await harness.type('dates-from', '2026-03-29 02:30', 'measure_durations');
    expect(harness.element('[data-testid="dates-gap-problem"]').dataset['problem']).toBe('skipped');
  });

  it('spells an ISO duration and totals it', async () => {
    const harness = await renderTool(DurationsComponent, answering(ANSWER));

    await harness.type('dates-duration', 'PT1H30M', 'measure_durations');

    expect(text(harness, '[data-testid="dates-duration-words"]')).toBe('1 heure 30 minutes');
    expect(rowValue(harness, 'data-total', 'seconds')).toBe('5 400');
    expect(rowValue(harness, 'data-total', 'minutes')).toBe('90');
    expect(rowValue(harness, 'data-total', 'hours')).toBe('1,5');
    expect(harness.element('[data-total="iso"]')).toBeNull();
  });

  it('gives the ISO form of words, and no total for a month', async () => {
    const harness = await renderTool(
      DurationsComponent,
      answering({
        ...ANSWER,
        duration: {
          kind: 'read',
          negative: false,
          parts: { years: 0, months: 1, weeks: 0, days: 0, hours: 1.5, minutes: 0, seconds: 0 },
          iso: 'P1MT1.5H',
          fromWords: true,
          totals: null,
        },
      }),
    );

    await harness.type('dates-duration', '1 mois 1.5h', 'measure_durations');

    expect(text(harness, '[data-testid="dates-duration-words"]')).toBe('1 mois 1,5 heure');
    expect(rowValue(harness, 'data-total', 'iso')).toBe('P1MT1.5H');
    expect(harness.element('[data-total="seconds"]')).toBeNull();
    expect(harness.element('[data-testid="dates-no-totals"]')).not.toBeNull();
  });

  it('says where a duration stops making sense', async () => {
    const harness = await renderTool(
      DurationsComponent,
      answering({ ...ANSWER, duration: { kind: 'unreadable', at: 7, problem: 'outOfOrder' } }),
    );

    await harness.type('dates-duration', 'PT30M1H', 'measure_durations');

    expect(text(harness, '[data-testid="dates-duration-problem"]')).toMatch(
      /^Caractère 7 : cette unité vient avant/,
    );
  });

  it('puts the present local time in « À »', async () => {
    const harness = await renderTool(DurationsComponent, answering(ANSWER));

    harness.element<HTMLButtonElement>('[data-testid="dates-to-now"]').click();
    await vi.waitFor(() => expect(harness.tools.requestsOf('measure_durations')).toHaveLength(1));

    expect(harness.element<HTMLInputElement>('[data-testid="dates-to"]').value).toMatch(
      /^\d{4}-\d{2}-\d{2} \d{2}:\d{2}$/,
    );
  });

  it('keeps the gap and the duration as a note in forms any language reads', async () => {
    const harness = await renderTool(DurationsComponent, answering(ANSWER));

    await harness.type('dates-from', '2026-03-12 09:00', 'measure_durations');
    await harness.type('dates-to', '2026-10-04 16:41', 'measure_durations');

    expect(harness.tool.result()).toEqual({
      title: { key: 'tools.dates.noteDurations' },
      kind: 'snippet',
      language: 'txt',
      content: '2026-03-12 09:00 → 2026-10-04 16:41\nP6M22DT7H41M\nPT1H30M = 5400 s',
    });
  });
});
