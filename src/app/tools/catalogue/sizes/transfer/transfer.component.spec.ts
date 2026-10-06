import { describe, expect, it, vi } from 'vitest';
import { TransferAnswer, TransferTime } from '@core/model/tool-answers.model';
import { FakeToolsRepository } from '@testing/fake-tools-repository';
import { ToolHarness, renderTool } from '@testing/tool-harness';
import { TransferComponent } from './transfer.component';

const time = (hours: number, minutes: number, seconds: number, days = 0): TransferTime => ({
  days,
  hours,
  minutes,
  seconds,
  underASecond: false,
});

const ESTIMATED: TransferAnswer = {
  kind: 'estimated',
  estimate: time(0, 6, 58),
  theoretical: time(0, 6, 16),
  efficiency: 90,
  connections: [
    { connection: 'fibre', rate: '1000000000', span: time(0, 0, 42) },
    { connection: null, rate: '100000000', span: time(0, 6, 58) },
    { connection: 'mobile4g', rate: '30000000', span: time(0, 23, 13) },
    { connection: 'adsl', rate: '10000000', span: time(1, 9, 38) },
  ],
};

describe('TransferComponent', () => {
  const answering =
    (answer: TransferAnswer) =>
    (tools: FakeToolsRepository): void => {
      tools.transfer = answer;
    };

  const asked = (harness: ToolHarness<TransferComponent>) => harness.tools.requestsOf('estimate_transfer');
  const text = (harness: ToolHarness<TransferComponent>, selector: string) =>
    harness.element(selector)?.textContent?.replace(/\s+/g, ' ').trim();

  async function fill(harness: ToolHarness<TransferComponent>): Promise<void> {
    const size = harness.element<HTMLInputElement>('[data-testid="transfer-size"]');
    size.value = '4,7';
    size.dispatchEvent(new Event('input'));
    await harness.type('transfer-rate', '100', 'estimate_transfer');
  }

  it('asks nothing until both the size and the rate are typed', async () => {
    const harness = await renderTool(TransferComponent, answering(ESTIMATED));
    const size = harness.element<HTMLInputElement>('[data-testid="transfer-size"]');
    size.value = '4,7';
    size.dispatchEvent(new Event('input'));
    await harness.settle();

    expect(asked(harness)).toEqual([]);
    expect(harness.element('.empty')).not.toBeNull();
  });

  it('says how long, at the rate and in theory, then by connection', async () => {
    const harness = await renderTool(TransferComponent, answering(ESTIMATED));

    await fill(harness);

    expect(asked(harness).at(-1)).toEqual({
      size: '4,7',
      sizeUnit: 'gigabyte',
      rate: '100',
      rateUnit: 'megabitPerSecond',
      efficiency: 90,
    });
    expect(text(harness, '[data-testid="transfer-time"]')).toBe('6 min 58 s');
    expect(text(harness, '[data-testid="transfer-theoretical"]')).toBe(
      '6 min 16 s en théorie, à plein débit.',
    );
    expect(text(harness, '[data-testid="transfer-estimate"] .estimate-caption')).toBe(
      'Durée estimée à 100 Mbit/s, rendement 90 %',
    );
    const rows = harness
      .all('[data-testid="output-row"]')
      .map((row) => [row.dataset['name'], row.querySelector('[data-testid="output-value"]')?.textContent]);
    expect(rows).toEqual([
      ['Fibre · 1 Gbit/s', '42 s'],
      ['100 Mbit/s', '6 min 58 s'],
      ['4G · 30 Mbit/s', '23 min 13 s'],
      ['ADSL · 10 Mbit/s', '1 h 9 min 38 s'],
    ]);
  });

  it('keeps the slider and the field on one value, held between 50 and 100', async () => {
    const harness = await renderTool(TransferComponent, answering(ESTIMATED));
    await fill(harness);
    const slider = harness.element<HTMLInputElement>('[data-testid="transfer-efficiency-slider"]');
    const field = harness.element<HTMLInputElement>('[data-testid="transfer-efficiency"]');

    slider.value = '75';
    slider.dispatchEvent(new Event('input'));
    await vi.waitFor(() => expect(asked(harness).at(-1)).toMatchObject({ efficiency: 75 }));
    await harness.settle();
    expect(field.value).toBe('75');

    field.value = '20';
    field.dispatchEvent(new Event('change'));
    await vi.waitFor(() => expect(asked(harness).at(-1)).toMatchObject({ efficiency: 50 }));
    await harness.settle();
    expect(slider.value).toBe('50');
  });

  it('spells days past a year, and less than a second', async () => {
    const harness = await renderTool(
      TransferComponent,
      answering({
        ...ESTIMATED,
        estimate: time(14, 13, 20, 92_592_592),
        theoretical: { ...time(0, 0, 0), underASecond: true },
      }),
    );

    await fill(harness);

    expect(text(harness, '[data-testid="transfer-time"]')).toBe('92 592 592 j 14 h 13 min 20 s');
    expect(text(harness, '[data-testid="transfer-theoretical"]')).toMatch(/^< 1 s en théorie/);
  });

  it('says which field is wrong, and that a rate of nothing carries nothing', async () => {
    const harness = await renderTool(
      TransferComponent,
      answering({ kind: 'unreadable', field: 'rate', at: 3 }),
    );

    await fill(harness);
    expect(text(harness, '[data-testid="transfer-problem"]')).toBe(
      'Débit : illisible à partir du caractère 3.',
    );

    harness.tools.transfer = { kind: 'zeroRate' };
    await harness.type('transfer-rate', '0', 'estimate_transfer');
    expect(text(harness, '[data-testid="transfer-problem"]')).toBe('Un débit nul ne transfère rien.');
  });

  it('keeps the connections as a note, aligned', async () => {
    const harness = await renderTool(TransferComponent, answering(ESTIMATED));

    await fill(harness);

    expect(harness.tool.result()).toEqual({
      title: { key: 'tools.sizes.noteTransfer', params: { size: '4,7', rate: '100 Mbit/s', efficiency: 90 } },
      kind: 'snippet',
      language: 'txt',
      content: [
        'Fibre · 1 Gbit/s  42 s',
        '100 Mbit/s        6 min 58 s',
        '4G · 30 Mbit/s    23 min 13 s',
        'ADSL · 10 Mbit/s  1 h 9 min 38 s',
      ].join('\n'),
    });
  });
});
