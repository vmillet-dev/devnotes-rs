import { TranslocoService } from '@jsverse/transloco';
import { describe, expect, it, vi } from 'vitest';
import { CronAnswer } from '@core/model/tool-answers.model';
import { FakeToolsRepository } from '@testing/fake-tools-repository';
import { ToolHarness, renderTool } from '@testing/tool-harness';
import { CronToolComponent, shortcutWindow } from './cron-tool.component';

const soon = Date.now() + 2 * 3_600_000 + 60_000;

const READ: CronAnswer = {
  kind: 'read',
  expanded: null,
  fields: [
    { field: 'minute', text: '*/15', start: 0, end: 4, pieces: [{ kind: 'step', step: 15 }] },
    { field: 'hour', text: '9-18', start: 5, end: 9, pieces: [{ kind: 'range', from: 9, to: 18 }] },
    { field: 'dayOfMonth', text: '*', start: 10, end: 11, pieces: [{ kind: 'every' }] },
    { field: 'month', text: '*', start: 12, end: 13, pieces: [{ kind: 'every' }] },
    { field: 'dayOfWeek', text: 'MON-FRI', start: 14, end: 21, pieces: [{ kind: 'range', from: 1, to: 5 }] },
  ],
  bothDays: false,
  zone: 'Europe/Paris',
  runs: [
    { date: '2026-09-30', time: '09:00:00', weekday: 3, offset: '+02:00', epochMilliseconds: soon },
    { date: '2026-09-30', time: '09:15:00', weekday: 3, offset: '+02:00', epochMilliseconds: null },
  ],
  check: null,
  window: null,
  // A Monday, as the zone reads it.
  now: { date: '2026-10-12', time: '10:00:00', weekday: 1, offset: '+02:00', epochMilliseconds: null },
};

const at = (date: string, time: string, weekday: number) => ({
  date,
  time,
  weekday,
  offset: '+02:00',
  epochMilliseconds: null,
});

/** Friday afternoon at every five minutes: seventy-two runs, the first and the last five handed over. */
const FRIDAY: CronAnswer = {
  ...READ,
  window: {
    kind: 'counted',
    from: at('2026-10-16', '12:00:00', 5),
    until: at('2026-10-16', '18:00:00', 5),
    total: 72,
    first: [at('2026-10-16', '12:00:00', 5), at('2026-10-16', '12:05:00', 5)],
    last: [at('2026-10-16', '17:50:00', 5), at('2026-10-16', '17:55:00', 5)],
  },
};

describe('CronToolComponent', () => {
  const answering =
    (answer: CronAnswer) =>
    (tools: FakeToolsRepository): void => {
      tools.cron = answer;
    };

  const asked = (harness: ToolHarness<CronToolComponent>) => harness.tools.requestsOf('describe_cron');
  const text = (harness: ToolHarness<CronToolComponent>, testid: string) =>
    harness.element(`[data-testid="${testid}"]`)?.textContent?.trim();

  it('asks nothing of an empty field', async () => {
    const harness = await renderTool(CronToolComponent);

    expect(asked(harness)).toEqual([]);
    expect(harness.tool.result()).toBeNull();
    expect(text(harness, 'cron-zone')).toBe('ce poste');
  });

  it('reads an expression aloud, field by field, with its next runs', async () => {
    const harness = await renderTool(CronToolComponent, answering(READ));

    await harness.type('cron-input', '*/15 9-18 * * MON-FRI', 'describe_cron');

    expect(asked(harness)).toEqual([
      { expression: '*/15 9-18 * * MON-FRI', zone: null, check: '', from: '', until: '' },
    ]);
    expect(text(harness, 'cron-sentence')).toBe('Toutes les 15 minutes, de 9 h à 18 h, du lundi au vendredi');
    expect(harness.all('[data-testid="cron-field"]').map((field) => field.dataset['field'])).toEqual([
      'minute',
      'hour',
      'dayOfMonth',
      'month',
      'dayOfWeek',
    ]);
    const runs = harness.all('[data-testid="cron-run"]');
    expect(runs).toHaveLength(2);
    expect(runs[0]?.textContent).toContain('mercredi');
    expect(runs[0]?.textContent).toContain('2026-09-30 09:00:00');
    expect(runs[0]?.textContent).toContain('dans 2 heures');
    expect(text(harness, 'cron-zone')).toBe('Europe/Paris');
    expect(harness.element('[data-testid="cron-either-day"]')).toBeNull();
  });

  it('lights the field the caret stands in', async () => {
    const harness = await renderTool(CronToolComponent, answering(READ));
    await harness.type('cron-input', '*/15 9-18 * * MON-FRI', 'describe_cron');
    const input = harness.element<HTMLInputElement>('[data-testid="cron-input"]');

    input.setSelectionRange(7, 7);
    input.dispatchEvent(new Event('click'));
    await harness.settle();
    expect(harness.element('[data-testid="cron-field"].lit')?.dataset['field']).toBe('hour');

    input.setSelectionRange(17, 17);
    input.dispatchEvent(new KeyboardEvent('keyup'));
    await harness.settle();
    expect(harness.element('[data-testid="cron-field"].lit')?.dataset['field']).toBe('dayOfWeek');

    input.dispatchEvent(new Event('blur'));
    await harness.settle();
    expect(harness.element('[data-testid="cron-field"].lit')).toBeNull();
  });

  it('writes the sentence again in the language chosen', async () => {
    const harness = await renderTool(CronToolComponent, answering(READ));
    await harness.type('cron-input', '*/15 9-18 * * MON-FRI', 'describe_cron');

    harness.fixture.debugElement.injector.get(TranslocoService).setActiveLang('en');
    await harness.settle();

    expect(text(harness, 'cron-sentence')).toBe(
      'Every 15 minutes, from 9:00 to 18:00, from Monday to Friday',
    );
  });

  it('shows what a macro stands for and lights no field of it', async () => {
    const harness = await renderTool(
      CronToolComponent,
      answering({ ...READ, expanded: '0 0 * * *', bothDays: true, runs: [] }),
    );
    await harness.type('cron-input', '@daily', 'describe_cron');
    const input = harness.element<HTMLInputElement>('[data-testid="cron-input"]');
    input.setSelectionRange(1, 1);
    input.dispatchEvent(new Event('click'));
    await harness.settle();

    expect(text(harness, 'cron-expanded')).toContain('0 0 * * *');
    expect(harness.element('[data-testid="cron-field"].lit')).toBeNull();
    expect(harness.element('[data-testid="cron-either-day"]')).not.toBeNull();
    expect(harness.element('[data-testid="cron-no-runs"]')).not.toBeNull();
  });

  it('says what @reboot means, and names the field a refusal is in', async () => {
    const harness = await renderTool(CronToolComponent, answering({ kind: 'reboot' }));
    await harness.type('cron-input', '@reboot', 'describe_cron');
    expect(harness.element('[data-testid="cron-reboot"]')).not.toBeNull();

    harness.tools.cron = { kind: 'refused', field: 'minute', token: '61', at: 1, problem: 'outOfRange' };
    await harness.type('cron-input', '61 * * * *', 'describe_cron');
    const problem = harness.element('[data-testid="cron-problem"]');
    expect(problem.dataset['field']).toBe('minute');
    expect(problem.textContent).toContain('Minute');
    expect(problem.textContent).toContain('Caractère 1');
    expect(problem.textContent).toContain('« 61 » sort des valeurs de ce champ : 0–59.');

    harness.tools.cron = { kind: 'refused', field: null, token: '*', at: 13, problem: 'fieldCount' };
    await harness.type('cron-input', '*/15 9-18 * *', 'describe_cron');
    expect(harness.element('[data-testid="cron-problem"]').textContent).toContain('cinq champs');
  });

  it('reads the expression in another zone, and back in the machine’s', async () => {
    const harness = await renderTool(CronToolComponent, (tools) => {
      tools.cron = READ;
      tools.zonesFound = [{ zone: 'Asia/Tokyo', city: 'Tokyo', offset: '+09:00', abbreviation: 'JST' }];
    });
    await harness.type('cron-input', '0 9 * * *', 'describe_cron');

    await harness.type('cron-zone-search', 'tok', 'search_time_zones');
    harness.element<HTMLButtonElement>('[data-testid="cron-zone-found"]').click();
    await vi.waitFor(() => expect(asked(harness)).toHaveLength(2));
    await harness.settle();
    expect(asked(harness)[1]).toMatchObject({ zone: 'Asia/Tokyo' });
    expect(harness.element<HTMLInputElement>('[data-testid="cron-zone-search"]').value).toBe('');

    harness.element<HTMLButtonElement>('[data-testid="cron-zone-local"]').click();
    await vi.waitFor(() => expect(asked(harness)).toHaveLength(3));
    expect(asked(harness)[2]).toMatchObject({ zone: null });
  });

  it('says a zone it does not know', async () => {
    const harness = await renderTool(
      CronToolComponent,
      answering({ kind: 'unknownZone', zone: 'Nowhere/City' }),
    );

    await harness.type('cron-input', '* * * * *', 'describe_cron');

    expect(text(harness, 'cron-unknown-zone')).toContain('Nowhere/City');
  });

  it('checks a date against the expression', async () => {
    const harness = await renderTool(
      CronToolComponent,
      answering({
        ...READ,
        check: { kind: 'checked', matches: true, at: '2026-09-30 09:15:00', before: null, after: null },
      }),
    );
    await harness.type('cron-input', '*/15 9-18 * * MON-FRI', 'describe_cron');
    await harness.type('cron-check', '2026-09-30 09:15', 'describe_cron');

    expect(asked(harness).at(-1)).toMatchObject({ check: '2026-09-30 09:15' });
    expect(harness.element('[data-testid="cron-verdict"]').dataset['verdict']).toBe('true');
    expect(text(harness, 'cron-verdict')).toBe('Oui : 2026-09-30 09:15:00 correspond.');
    // A yes leaves nothing to ask.
    expect(harness.element('[data-testid="cron-nearest"]')).toBeNull();

    for (const [check, verdict] of [
      [{ kind: 'checked', matches: false, at: '2026-09-27 09:15:00', before: null, after: null }, 'false'],
      [{ kind: 'unreadable', at: 6 }, 'unreadable'],
      [{ kind: 'skipped' }, 'skipped'],
    ] as const) {
      harness.tools.cron = { ...READ, check };
      await harness.type('cron-check', `${verdict} date`, 'describe_cron');
      expect(harness.element('[data-testid="cron-verdict"]').dataset['verdict']).toBe(verdict);
    }
  });

  /** A « no » left one asking when, then. */
  it('says the nearest run on either side of a date that does not match', async () => {
    const harness = await renderTool(
      CronToolComponent,
      answering({
        ...READ,
        check: {
          kind: 'checked',
          matches: false,
          at: '2026-09-27 09:15:00',
          before: at('2026-09-25', '18:45:00', 5),
          after: at('2026-09-28', '09:00:00', 1),
        },
      }),
    );
    await harness.type('cron-input', '*/15 9-18 * * MON-FRI', 'describe_cron');
    await harness.type('cron-check', '2026-09-27 09:15', 'describe_cron');

    expect(text(harness, 'cron-nearest-before')).toBe('Exécution la plus proche avant : 2026-09-25 18:45:00');
    expect(text(harness, 'cron-nearest-after')).toBe('Exécution la plus proche après : 2026-09-28 09:00:00');
  });

  /**
   * « Will this run on Friday afternoon? » had no answer: the list was the next ten from now,
   * and the check one exact minute.
   */
  it('lists the runs between two moments, counted, the first and the last around how many between', async () => {
    const harness = await renderTool(CronToolComponent, answering(FRIDAY));
    await harness.type('cron-input', '*/5 * * * 1-5', 'describe_cron');
    await harness.type('cron-from', '2026-10-16 12:00', 'describe_cron');
    await harness.type('cron-until', '2026-10-16 18:00', 'describe_cron');

    expect(asked(harness).at(-1)).toMatchObject({ from: '2026-10-16 12:00', until: '2026-10-16 18:00' });
    const count = harness.element('[data-testid="cron-window-count"]');
    expect(count.dataset['total']).toBe('72');
    expect(count.textContent?.trim()).toBe(
      '72 exécutions entre vendredi 2026-10-16 12:00 et vendredi 2026-10-16 18:00',
    );
    // In the order they run: the first, how many are not listed, the last.
    expect(
      harness
        .all('[data-testid="cron-run-at"], [data-testid="cron-window-gap"]')
        .map((row) => row.textContent?.trim()),
    ).toEqual([
      '2026-10-16 12:00:00',
      '2026-10-16 12:05:00',
      '… 68 autres …',
      '2026-10-16 17:50:00',
      '2026-10-16 17:55:00',
    ]);
  });

  it('lists every run of a window that holds few, with no cut', async () => {
    const window = FRIDAY.kind === 'read' && FRIDAY.window?.kind === 'counted' ? FRIDAY.window : null;
    const harness = await renderTool(
      CronToolComponent,
      answering({ ...READ, window: { ...window!, total: 2, last: [] } }),
    );
    await harness.type('cron-input', '0 12 * * *', 'describe_cron');

    expect(harness.all('[data-testid="cron-run"]')).toHaveLength(2);
    expect(harness.element('[data-testid="cron-window-gap"]')).toBeNull();
    expect(text(harness, 'cron-window-count')).toContain('2 exécutions entre');
  });

  /** The answer one came for: a Saturday, for an expression of the working days. */
  it('says a window holds no run, and lists none', async () => {
    const window = FRIDAY.kind === 'read' && FRIDAY.window?.kind === 'counted' ? FRIDAY.window : null;
    const harness = await renderTool(
      CronToolComponent,
      answering({ ...READ, window: { ...window!, total: 0, first: [], last: [] } }),
    );
    await harness.type('cron-input', '*/5 * * * 1-5', 'describe_cron');

    expect(text(harness, 'cron-window-count')).toBe('Aucune exécution dans cet intervalle.');
    expect(harness.element('[data-testid="cron-window-count"]').dataset['total']).toBe('0');
    expect(harness.all('[data-testid="cron-run"]')).toHaveLength(0);
    expect(harness.element('[data-testid="cron-no-runs"]')).toBeNull();
  });

  it('says why a window is not counted', async () => {
    const harness = await renderTool(CronToolComponent, answering(READ));
    await harness.type('cron-input', '0 9 * * *', 'describe_cron');
    expect(harness.element('[data-testid="cron-window-problem"]')).toBeNull();

    for (const [window, said] of [
      [{ kind: 'unreadable', end: 'from', at: 6 }, 'Le début ne se lit pas, au caractère 6.'],
      [{ kind: 'unreadable', end: 'until', at: 3 }, 'La fin ne se lit pas, au caractère 3.'],
      [{ kind: 'skipped', end: 'from' }, 'Ce début n’existe pas dans ce fuseau.'],
      [{ kind: 'skipped', end: 'until' }, 'Cette fin n’existe pas dans ce fuseau.'],
      [{ kind: 'backwards' }, 'La fin doit venir après le début.'],
      [{ kind: 'tooLong', years: 10 }, 'Intervalle trop long : 10 ans au plus.'],
    ] as const) {
      harness.tools.cron = { ...READ, window };
      await harness.type('cron-until', said, 'describe_cron');

      expect(text(harness, 'cron-window-problem')).toBe(said);
      expect(harness.element('[data-testid="cron-window-problem"]').dataset['problem']).toBe(window.kind);
      // The ten from now stay under it.
      expect(harness.all('[data-testid="cron-run"]')).toHaveLength(2);
    }
  });

  it('fills the two ends from a shortcut, counted from what now is in the zone', async () => {
    const harness = await renderTool(CronToolComponent, answering(READ));
    expect(harness.all('[data-testid="cron-shortcut"]')).toHaveLength(0);
    await harness.type('cron-input', '0 9 * * *', 'describe_cron');
    const shortcut = (id: string) =>
      harness.element<HTMLButtonElement>(`[data-testid="cron-shortcut"][data-shortcut="${id}"]`);
    const ends = () =>
      ['cron-from', 'cron-until'].map(
        (testid) => harness.element<HTMLInputElement>(`[data-testid="${testid}"]`).value,
      );
    expect(harness.all('[data-testid="cron-shortcut"]').map((button) => button.textContent?.trim())).toEqual([
      'aujourd’hui',
      'demain',
      'ce week-end',
      'vendredi après-midi',
    ]);

    shortcut('fridayAfternoon').click();
    await vi.waitFor(() =>
      expect(asked(harness).at(-1)).toMatchObject({ from: '2026-10-16 12:00', until: '2026-10-16 18:00' }),
    );
    await harness.settle();
    expect(ends()).toEqual(['2026-10-16 12:00', '2026-10-16 18:00']);

    shortcut('today').click();
    await harness.settle();
    expect(ends()).toEqual(['2026-10-12 00:00', '2026-10-13 00:00']);
  });

  it('counts a shortcut from the day it is: the weekend under way, the next Friday, a month that ends', () => {
    const [monday, friday, saturday, sunday] = [
      { date: '2026-10-12', weekday: 1 },
      { date: '2026-10-16', weekday: 5 },
      { date: '2026-10-17', weekday: 6 },
      { date: '2026-10-18', weekday: 7 },
    ];

    expect(shortcutWindow('today', monday)).toEqual(['2026-10-12 00:00', '2026-10-13 00:00']);
    expect(shortcutWindow('tomorrow', monday)).toEqual(['2026-10-13 00:00', '2026-10-14 00:00']);
    expect(shortcutWindow('weekend', monday)).toEqual(['2026-10-17 00:00', '2026-10-19 00:00']);
    expect(shortcutWindow('fridayAfternoon', monday)).toEqual(['2026-10-16 12:00', '2026-10-16 18:00']);
    // On the Friday itself, this afternoon.
    expect(shortcutWindow('fridayAfternoon', friday)).toEqual(['2026-10-16 12:00', '2026-10-16 18:00']);
    // On a Saturday or a Sunday, the weekend under way and the Friday that follows it.
    for (const day of [saturday, sunday]) {
      expect(shortcutWindow('weekend', day)).toEqual(['2026-10-17 00:00', '2026-10-19 00:00']);
      expect(shortcutWindow('fridayAfternoon', day)).toEqual(['2026-10-23 12:00', '2026-10-23 18:00']);
    }
    expect(shortcutWindow('tomorrow', { date: '2026-12-31', weekday: 4 })).toEqual([
      '2027-01-01 00:00',
      '2027-01-02 00:00',
    ]);
  });

  it('keeps the expression, its sentence and its runs as a note', async () => {
    const harness = await renderTool(CronToolComponent, answering(READ));
    await harness.type('cron-input', ' */15 9-18 * * MON-FRI ', 'describe_cron');

    expect(harness.tool.result()).toEqual({
      title: { key: 'tools.cron.noteTitle', params: { expression: '*/15 9-18 * * MON-FRI' } },
      kind: 'snippet',
      language: 'txt',
      content: [
        '*/15 9-18 * * MON-FRI',
        'Toutes les 15 minutes, de 9 h à 18 h, du lundi au vendredi.',
        '',
        'Prochaines exécutions (Europe/Paris) :',
        '2026-09-30 09:00:00',
        '2026-09-30 09:15:00',
      ].join('\n'),
    });

    harness.tools.cron = { ...READ, runs: [] };
    await harness.type('cron-input', '0 0 30 2 *', 'describe_cron');
    expect(harness.tool.result()?.content).toBe(
      '0 0 30 2 *\nToutes les 15 minutes, de 9 h à 18 h, du lundi au vendredi.',
    );
  });

  it('empties everything but the zone on Vider', async () => {
    const harness = await renderTool(CronToolComponent, answering(READ));
    await harness.type('cron-input', '*/15 9-18 * * MON-FRI', 'describe_cron');
    await harness.type('cron-check', '2026-09-30 09:15', 'describe_cron');
    await harness.type('cron-from', '2026-10-16 12:00', 'describe_cron');
    await harness.type('cron-until', '2026-10-16 18:00', 'describe_cron');

    harness.tool.clear();
    await harness.settle();

    expect(harness.element<HTMLInputElement>('[data-testid="cron-input"]').value).toBe('');
    expect(harness.element<HTMLInputElement>('[data-testid="cron-check"]').value).toBe('');
    expect(harness.element<HTMLInputElement>('[data-testid="cron-from"]').value).toBe('');
    expect(harness.element<HTMLInputElement>('[data-testid="cron-until"]').value).toBe('');
    expect(harness.element('[data-testid="cron-sentence"]')).toBeNull();
  });
});
