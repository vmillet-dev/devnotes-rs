import { TranslocoService } from '@jsverse/transloco';
import { describe, expect, it, vi } from 'vitest';
import { CronAnswer } from '@core/model/tool-answers.model';
import { FakeToolsRepository } from '@testing/fake-tools-repository';
import { ToolHarness, renderTool } from '@testing/tool-harness';
import { CronToolComponent } from './cron-tool.component';

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

    expect(asked(harness)).toEqual([{ expression: '*/15 9-18 * * MON-FRI', zone: null, check: '' }]);
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
      answering({ ...READ, check: { kind: 'checked', matches: true, at: '2026-09-30 09:15:00' } }),
    );
    await harness.type('cron-input', '*/15 9-18 * * MON-FRI', 'describe_cron');
    await harness.type('cron-check', '2026-09-30 09:15', 'describe_cron');

    expect(asked(harness).at(-1)).toMatchObject({ check: '2026-09-30 09:15' });
    expect(harness.element('[data-testid="cron-verdict"]').dataset['verdict']).toBe('true');
    expect(text(harness, 'cron-verdict')).toBe('Oui : 2026-09-30 09:15:00 correspond.');

    for (const [check, verdict] of [
      [{ kind: 'checked', matches: false, at: '2026-09-27 09:15:00' }, 'false'],
      [{ kind: 'unreadable', at: 6 }, 'unreadable'],
      [{ kind: 'skipped' }, 'skipped'],
    ] as const) {
      harness.tools.cron = { ...READ, check };
      await harness.type('cron-check', `${verdict} date`, 'describe_cron');
      expect(harness.element('[data-testid="cron-verdict"]').dataset['verdict']).toBe(verdict);
    }
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

    harness.tool.clear();
    await harness.settle();

    expect(harness.element<HTMLInputElement>('[data-testid="cron-input"]').value).toBe('');
    expect(harness.element<HTMLInputElement>('[data-testid="cron-check"]').value).toBe('');
    expect(harness.element('[data-testid="cron-sentence"]')).toBeNull();
  });
});
