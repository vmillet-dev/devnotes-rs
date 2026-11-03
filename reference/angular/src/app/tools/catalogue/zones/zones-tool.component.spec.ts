import { TestBed } from '@angular/core/testing';
import { describe, expect, it, vi } from 'vitest';
import { ZoneTime, ZonesAnswer } from '@core/model/tool-answers.model';
import { SettingsStore } from '@core/services/settings/settings.store';
import { FakeToolsRepository } from '@testing/fake-tools-repository';
import { ToolHarness, renderTool } from '@testing/tool-harness';
import { ZonesToolComponent } from './zones-tool.component';

const time = (zone: string, overrides: Partial<ZoneTime> = {}): ZoneTime => ({
  zone,
  city: zone.split('/').at(-1)!.replace('_', ' '),
  local: false,
  source: false,
  date: '2026-09-29',
  time: '20:30:00',
  iso: '2026-09-29T20:30:00+02:00',
  offset: '+02:00',
  abbreviation: 'CEST',
  summerTime: true,
  dayShift: 0,
  ...overrides,
});

const PLACED: ZonesAnswer = {
  kind: 'placed',
  from: 'Europe/Paris',
  ambiguous: null,
  times: [
    time('Europe/Paris', { local: true, source: true }),
    time('UTC', { time: '18:30:00', offset: '+00:00', abbreviation: 'UTC', summerTime: false }),
    time('America/New_York', { time: '14:30:00', offset: '-04:00', abbreviation: 'EDT' }),
    time('Asia/Tokyo', {
      date: '2026-09-30',
      time: '03:30:00',
      offset: '+09:00',
      abbreviation: 'JST',
      summerTime: false,
      dayShift: 1,
    }),
    time('Pacific/Honolulu', { date: '2026-09-28', dayShift: -1, abbreviation: null, summerTime: false }),
  ],
  unknown: [],
};

describe('ZonesToolComponent', () => {
  const answering =
    (answer: ZonesAnswer) =>
    (tools: FakeToolsRepository): void => {
      tools.zonesAnswer = answer;
    };

  const placements = (harness: ToolHarness<ZonesToolComponent>) => harness.tools.requestsOf('place_in_zones');

  const row = (harness: ToolHarness<ZonesToolComponent>, zone: string) =>
    harness.element(`[data-testid="zones-row"][data-zone="${zone}"]`);

  it('asks nothing of an empty field', async () => {
    const harness = await renderTool(ZonesToolComponent);

    expect(placements(harness)).toEqual([]);
    expect(harness.element('.empty')).not.toBeNull();
    expect(harness.tool.result()).toBeNull();
  });

  it('places a time in the machine’s zone and in the zones of the list', async () => {
    const harness = await renderTool(ZonesToolComponent, answering(PLACED));

    await harness.type('zones-input', '2026-09-29 20:30', 'place_in_zones');

    expect(placements(harness)).toEqual([
      {
        text: '2026-09-29 20:30',
        from: null,
        zones: ['UTC', 'America/New_York', 'Asia/Tokyo'],
        later: false,
      },
    ]);
    expect(harness.all('[data-testid="zones-row"]').map((row) => row.dataset['zone'])).toEqual([
      'Europe/Paris',
      'UTC',
      'America/New_York',
      'Asia/Tokyo',
      'Pacific/Honolulu',
    ]);
    const tokyo = row(harness, 'Asia/Tokyo');
    expect(tokyo.querySelector('[data-testid="zones-time"]')?.textContent).toBe('03:30:00');
    expect(tokyo.querySelector('[data-testid="zones-shift"]')?.textContent?.trim()).toBe('+1 j');
    expect(tokyo.querySelector('[data-testid="zones-summer"]')).toBeNull();
    expect(
      row(harness, 'Pacific/Honolulu').querySelector('[data-testid="zones-shift"]')?.textContent?.trim(),
    ).toBe('−1 j');
    expect(
      row(harness, 'Pacific/Honolulu').querySelector('[data-testid="zones-offset"]')?.textContent?.trim(),
    ).toBe('Pacific/Honolulu · UTC+02:00');
    expect(
      row(harness, 'America/New_York')
        .querySelector('[data-testid="zones-offset"]')
        ?.textContent?.replace(/\s+/g, ' ')
        .trim(),
    ).toBe('America/New_York · UTC-04:00 · EDT · heure d’été');
    const paris = row(harness, 'Europe/Paris');
    expect(paris.querySelector('[data-testid="zones-summer"]')).not.toBeNull();
    expect(paris.querySelector('[data-testid="zones-remove"]')).toBeNull();
    expect(paris.querySelector('[data-testid="zones-type-in"]')).toBeNull();
    expect(harness.element('[data-testid="zones-from"]').dataset['zone']).toBe('Europe/Paris');
    expect(harness.element('[data-testid="zones-from"]').textContent?.replace(/\s+/g, ' ').trim()).toBe(
      'Saisie à l’heure de Europe/Paris. Aucune ambiguïté d’heure d’été à cette date.',
    );
    expect(paris.querySelector('[data-testid="zones-local"]')).not.toBeNull();
    expect(paris.querySelector('[data-testid="zones-source"]')).not.toBeNull();
    expect(row(harness, 'UTC').querySelector('[data-testid="zones-local"]')).toBeNull();
  });

  /** The one thing a tool keeps on disk: the list is the application's preference. */
  it('keeps the list in the application’s preferences', async () => {
    const harness = await renderTool(ZonesToolComponent, (tools) => {
      tools.zonesAnswer = PLACED;
      tools.zonesFound = [{ zone: 'Asia/Kolkata', city: 'Kolkata', offset: '+05:30', abbreviation: 'IST' }];
    });
    const settings = TestBed.inject(SettingsStore);

    await harness.type('zones-search', 'kol', 'search_time_zones');
    harness.element<HTMLButtonElement>('[data-testid="zones-found-entry"]').click();
    await harness.settle();

    expect(settings.timeZones()).toEqual(['UTC', 'America/New_York', 'Asia/Tokyo', 'Asia/Kolkata']);
  });

  it('drops from the list a zone the database does not know', async () => {
    const harness = await renderTool(ZonesToolComponent, (tools) => {
      tools.zonesAnswer = { ...PLACED, unknown: ['Mars/Olympus'] };
    });
    const settings = TestBed.inject(SettingsStore);
    settings.timeZones.write(['UTC', 'Mars/Olympus']);

    await harness.type('zones-input', '2026-09-29 20:30', 'place_in_zones');

    await vi.waitFor(() => expect(settings.timeZones()).toEqual(['UTC']));
    expect(harness.element('[data-testid="zones-unknown"]').textContent).toContain('Mars/Olympus');
  });

  it('types the time in another zone of the list', async () => {
    const harness = await renderTool(ZonesToolComponent, answering(PLACED));
    await harness.type('zones-input', '2026-09-29 20:30', 'place_in_zones');

    row(harness, 'Asia/Tokyo').querySelector<HTMLButtonElement>('[data-testid="zones-type-in"]')!.click();
    await vi.waitFor(() => expect(placements(harness)).toHaveLength(2));

    expect(placements(harness)[1]).toMatchObject({ from: 'Asia/Tokyo' });
  });

  it('removes a zone of the list, and adds one found by its city', async () => {
    const harness = await renderTool(ZonesToolComponent, (tools) => {
      tools.zonesAnswer = PLACED;
      tools.zonesFound = [
        { zone: 'Asia/Kolkata', city: 'Kolkata', offset: '+05:30', abbreviation: 'IST' },
        { zone: 'Asia/Kathmandu', city: 'Kathmandu', offset: '+05:45', abbreviation: null },
      ];
    });
    await harness.type('zones-input', '2026-09-29 20:30', 'place_in_zones');

    row(harness, 'America/New_York')
      .querySelector<HTMLButtonElement>('[data-testid="zones-remove"]')!
      .click();
    await vi.waitFor(() => expect(placements(harness)).toHaveLength(2));
    await harness.type('zones-search', 'kol', 'search_time_zones');
    expect(harness.all('[data-testid="zones-found-entry"]')).toHaveLength(2);
    harness.element<HTMLButtonElement>('[data-testid="zones-found-entry"][data-zone="Asia/Kolkata"]').click();
    await vi.waitFor(() => expect(placements(harness)).toHaveLength(3));

    expect(placements(harness).map((request) => (request as { zones: string[] }).zones)).toEqual([
      ['UTC', 'America/New_York', 'Asia/Tokyo'],
      ['UTC', 'Asia/Tokyo'],
      ['UTC', 'Asia/Tokyo', 'Asia/Kolkata'],
    ]);
    expect(harness.element<HTMLInputElement>('[data-testid="zones-search"]').value).toBe('');
  });

  it('adds a zone already listed only once, and says when a search finds nothing', async () => {
    const harness = await renderTool(ZonesToolComponent, (tools) => {
      tools.zonesFound = [{ zone: 'UTC', city: 'UTC', offset: '+00:00', abbreviation: 'UTC' }];
    });

    await harness.type('zones-search', 'utc', 'search_time_zones');
    harness.element<HTMLButtonElement>('[data-testid="zones-found-entry"]').click();
    await harness.type('zones-input', '2026-09-29 20:30', 'place_in_zones');
    expect(placements(harness)[0]).toMatchObject({ zones: ['UTC', 'America/New_York', 'Asia/Tokyo'] });

    harness.tools.zonesFound = [];
    await harness.type('zones-search', 'mars', 'search_time_zones');
    expect(harness.element('[data-testid="zones-no-match"]').textContent).toContain('mars');
  });

  it('offers both readings of a repeated hour, and asks for the one chosen', async () => {
    const harness = await renderTool(
      ZonesToolComponent,
      answering({
        ...PLACED,
        ambiguous: [
          { offset: '+02:00', abbreviation: 'CEST', isoUtc: '2026-10-25T00:30:00Z' },
          { offset: '+01:00', abbreviation: null, isoUtc: '2026-10-25T01:30:00Z' },
        ],
        unknown: ['Nowhere/City'],
      }),
    );
    await harness.type('zones-input', '2026-10-25 02:30', 'place_in_zones');

    expect(harness.element('[data-testid="zones-reading-0"]').closest('label')?.textContent).toContain(
      'CEST',
    );
    expect(harness.element<HTMLInputElement>('[data-testid="zones-reading-0"]').checked).toBe(true);
    harness.element<HTMLInputElement>('[data-testid="zones-reading-1"]').dispatchEvent(new Event('change'));
    await vi.waitFor(() => expect(placements(harness)).toHaveLength(2));

    expect(placements(harness)[1]).toMatchObject({ later: true });
    expect(harness.element('[data-testid="zones-unknown"]').textContent).toContain('Nowhere/City');
  });

  it('says why a time cannot be placed', async () => {
    const harness = await renderTool(
      ZonesToolComponent,
      answering({ kind: 'skipped', from: 'Europe/Paris', before: '+01:00', after: '+02:00' }),
    );
    const problem = () => harness.element('[data-testid="zones-problem"]');

    await harness.type('zones-input', '2026-03-29 02:30', 'place_in_zones');
    expect(problem().textContent).toContain('UTC+01:00');
    expect(problem().textContent).toContain('UTC+02:00');

    for (const answer of [
      { kind: 'unreadable', at: 12 },
      { kind: 'unknownZone', zone: 'Mars/Olympus' },
      { kind: 'outOfRange' },
    ] as ZonesAnswer[]) {
      harness.tools.zonesAnswer = answer;
      await harness.type('zones-input', `${answer.kind} text`, 'place_in_zones');
      expect(problem().dataset['problem']).toBe(answer.kind);
    }
    expect(problem().textContent).toContain('Au-delà');
  });

  it('takes the wall clock of the zone typed in, from Rust', async () => {
    const harness = await renderTool(ZonesToolComponent, answering(PLACED));

    harness.element<HTMLButtonElement>('[data-testid="zones-now"]').click();
    await vi.waitFor(() => expect(placements(harness)).toHaveLength(1));

    expect(harness.tools.requestsOf('time_in_zone')).toEqual([null]);
    expect(harness.element<HTMLInputElement>('[data-testid="zones-input"]').value).toBe(
      '2026-09-29 14:03:12',
    );
  });

  it('leaves the field alone when the clock cannot be read', async () => {
    const harness = await renderTool(ZonesToolComponent);
    vi.spyOn(harness.tools, 'timeInZone').mockRejectedValue(new Error('no clock'));

    harness.element<HTMLButtonElement>('[data-testid="zones-now"]').click();
    await harness.settle();

    expect(harness.element<HTMLInputElement>('[data-testid="zones-input"]').value).toBe('');
  });

  it('keeps every zone’s time as a note, one line each', async () => {
    const harness = await renderTool(ZonesToolComponent, answering(PLACED));

    await harness.type('zones-input', ' 2026-09-29 20:30 ', 'place_in_zones');

    const result = harness.tool.result();
    expect(result?.title).toEqual({ key: 'tools.zones.noteTitle', params: { time: '2026-09-29 20:30' } });
    expect(result?.content.split('\n')[0]).toBe(
      'Europe/Paris              2026-09-29 20:30:00  UTC+02:00  CEST',
    );
    expect(result?.content.split('\n')[4]).toBe('Pacific/Honolulu          2026-09-28 20:30:00  UTC+02:00');
  });

  it('empties the field and the search on Vider, and keeps the list', async () => {
    const harness = await renderTool(ZonesToolComponent, answering(PLACED));
    await harness.type('zones-input', '2026-09-29 20:30', 'place_in_zones');
    await harness.type('zones-search', 'tok', 'search_time_zones');

    harness.tool.clear();
    await harness.settle();

    expect(harness.element<HTMLInputElement>('[data-testid="zones-input"]').value).toBe('');
    expect(harness.element<HTMLInputElement>('[data-testid="zones-search"]').value).toBe('');
    expect(harness.all('[data-testid="zones-row"]')).toHaveLength(0);
  });
});
