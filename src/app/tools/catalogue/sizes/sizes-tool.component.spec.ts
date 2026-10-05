import { TranslocoService } from '@jsverse/transloco';
import { describe, expect, it, vi } from 'vitest';
import { SizeUnit, SizesAnswer } from '@core/model/tool-answers.model';
import { FakeToolsRepository } from '@testing/fake-tools-repository';
import { ToolHarness, renderTool } from '@testing/tool-harness';
import { SizesToolComponent } from './sizes-tool.component';

const NNBSP = ' ';

const row = (unit: SizeUnit, exact: string, rounded = exact) => ({ unit, exact, rounded });

const ONE_AND_A_HALF_GB: SizesAnswer = {
  kind: 'converted',
  unit: 'gigabyte',
  unitInText: true,
  rows: [
    row('byte', '1500000000'),
    row('bit', '12000000000'),
    row('kilobit', '12000000'),
    row('megabit', '12000'),
    row('gigabit', '12'),
    row('terabit', '0.012'),
    row('kilobyte', '1500000'),
    row('megabyte', '1500'),
    row('gigabyte', '1.5'),
    row('terabyte', '0.0015', '0.002'),
    row('petabyte', '0.0000015', '0'),
    row('kibibyte', '1464843.75'),
    row('mebibyte', '1430.511474609375', '1430.511'),
    row('gibibyte', '1.39698386192321777343750', '1.397'),
    row('tebibyte', '0.00136424205265939235687255859375', '0.001'),
    row('pebibyte', '0.00000133226762955018784850836', '0'),
  ],
  gap: { decimal: 'gigabyte', binary: 'gibibyte', exact: '0.931322574615478515625', rounded: '0.931' },
};

describe('SizesToolComponent', () => {
  const answering =
    (answer: SizesAnswer) =>
    (tools: FakeToolsRepository): void => {
      tools.sizes = answer;
    };

  const asked = (harness: ToolHarness<SizesToolComponent>) => harness.tools.requestsOf('convert_size');
  const value = (harness: ToolHarness<SizesToolComponent>, unit: SizeUnit) =>
    harness.element(`[data-unit="${unit}"] [data-testid="output-value"]`)?.textContent;

  it('asks nothing of an empty field', async () => {
    const harness = await renderTool(SizesToolComponent);

    expect(asked(harness)).toEqual([]);
    expect(harness.element('.empty')).not.toBeNull();
    expect(harness.tool.result()).toBeNull();
  });

  it('shows a quantity in every unit, in French digits, rounded where it has to be', async () => {
    const harness = await renderTool(SizesToolComponent, answering(ONE_AND_A_HALF_GB));

    await harness.type('sizes-input', '1,5 Go', 'convert_size');

    expect(asked(harness)).toEqual([{ text: '1,5 Go', unit: 'megabyte', decimals: 3 }]);
    expect(value(harness, 'byte')).toBe(`1${NNBSP}500${NNBSP}000${NNBSP}000`);
    expect(value(harness, 'gigabyte')).toBe('1,5');
    expect(value(harness, 'gibibyte')).toBe('≈ 1,397');
    // Too small for three decimals: a power of ten rather than a bare 0.
    expect(value(harness, 'petabyte')).toBe('1,5 × 10⁻⁶');
    expect(value(harness, 'pebibyte')).toBe('≈ 1,332 × 10⁻⁶');
    expect(harness.element('[data-unit="gibibyte"]').textContent).toContain('Gio');
    expect(harness.all('[data-testid="sizes-group"]').map((title) => title.dataset['group'])).toEqual([
      'bytes',
      'decimal',
      'binary',
    ]);
    expect(harness.element('[data-testid="sizes-reading"]').dataset['unit']).toBe('gigabyte');
    expect(harness.element('[data-testid="sizes-unit-in-text"]')).not.toBeNull();
    expect(harness.element('[data-testid="sizes-gap"]').textContent?.trim()).toBe('1 Go = 0,931 Gio');
  });

  it('copies the exact value behind a rounded one', async () => {
    const harness = await renderTool(SizesToolComponent, answering(ONE_AND_A_HALF_GB));
    await harness.type('sizes-input', '1,5 Go', 'convert_size');

    harness.element<HTMLButtonElement>('[data-unit="gibibyte"] [data-testid="copy-value"]').click();

    await vi.waitFor(() => expect(harness.clipboard.content).toBe('1.39698386192321777343750'));
  });

  it('writes English symbols and digits in English', async () => {
    const harness = await renderTool(SizesToolComponent, answering(ONE_AND_A_HALF_GB));
    await harness.type('sizes-input', '1.5 GB', 'convert_size');

    harness.fixture.debugElement.injector.get(TranslocoService).setActiveLang('en');
    await harness.settle();

    expect(value(harness, 'byte')).toBe('1,500,000,000');
    expect(harness.element('[data-testid="sizes-gap"]').textContent?.trim()).toBe('1 GB = 0.931 GiB');
  });

  it('asks in the unit chosen for a number without one, and with the decimals set', async () => {
    const harness = await renderTool(
      SizesToolComponent,
      answering({ ...ONE_AND_A_HALF_GB, unitInText: false }),
    );
    await harness.type('sizes-input', '2048', 'convert_size');

    harness.element<HTMLButtonElement>('[data-testid="choice-sizes-unit"]').click();
    await harness.settle();
    harness.element<HTMLButtonElement>('[data-testid="choice-option"][data-option-id="kibibyte"]').click();
    await vi.waitFor(() => expect(asked(harness)).toHaveLength(2));
    await harness.type('sizes-decimals', '6', 'convert_size');
    await harness.type('sizes-decimals', '99', 'convert_size');
    const decimals = harness.element<HTMLInputElement>('[data-testid="sizes-decimals"]');
    decimals.value = '';
    decimals.dispatchEvent(new Event('input'));
    await harness.settle();

    expect(asked(harness)).toEqual([
      { text: '2048', unit: 'megabyte', decimals: 3 },
      { text: '2048', unit: 'kibibyte', decimals: 3 },
      { text: '2048', unit: 'kibibyte', decimals: 6 },
      { text: '2048', unit: 'kibibyte', decimals: 12 },
    ]);
    expect(harness.element('[data-testid="sizes-unit-in-text"]')).toBeNull();
  });

  it('says why a quantity cannot be read', async () => {
    const harness = await renderTool(SizesToolComponent, answering({ kind: 'unreadable', at: 3 }));
    const problem = () => harness.element('[data-testid="sizes-problem"]');

    await harness.type('sizes-input', '5 MX', 'convert_size');
    expect(problem().textContent).toContain('caractère 3');

    harness.tools.sizes = { kind: 'negative', at: 1 };
    await harness.type('sizes-input', '-5 MB', 'convert_size');
    expect(problem().dataset['problem']).toBe('negative');
    expect(problem().textContent).toContain('négative');

    harness.tools.sizes = { kind: 'tooLarge' };
    await harness.type('sizes-input', '1e30 PiB', 'convert_size');
    expect(problem().dataset['problem']).toBe('tooLarge');
    expect(harness.tool.result()).toBeNull();
  });

  it('keeps every unit’s exact value as a note', async () => {
    const harness = await renderTool(SizesToolComponent, answering(ONE_AND_A_HALF_GB));

    await harness.type('sizes-input', '1,5 Go', 'convert_size');

    const result = harness.tool.result();
    expect(result?.title).toEqual({ key: 'tools.sizes.noteTitle', params: { value: '1,5', unit: 'Go' } });
    expect(result?.content.split('\n')[0]).toBe('o     1500000000');
    expect(result?.content).toContain('Gio   1.39698386192321777343750');
  });

  it('empties the field on Vider and keeps the unit chosen', async () => {
    const harness = await renderTool(SizesToolComponent, answering(ONE_AND_A_HALF_GB));
    await harness.type('sizes-input', '1,5 Go', 'convert_size');

    harness.tool.clear();
    await harness.settle();

    expect(harness.element<HTMLInputElement>('[data-testid="sizes-input"]').value).toBe('');
    expect(harness.all('[data-testid="output-row"]')).toHaveLength(0);
  });

  describe('the Transfert tab', () => {
    const tab = async (harness: ToolHarness<SizesToolComponent>, id: 'conversion' | 'transfer') => {
      harness
        .element<HTMLButtonElement>(`[data-testid="segmented-sizes-tab"] [data-segment-id="${id}"]`)
        .click();
      await harness.settle();
    };

    it('fills both tabs from its sample, saves what the tab on screen shows, and empties both', async () => {
      const harness = await renderTool(SizesToolComponent, (tools) => {
        tools.sizes = ONE_AND_A_HALF_GB;
        tools.transfer = {
          kind: 'estimated',
          estimate: { days: 0, hours: 0, minutes: 6, seconds: 58, underASecond: false },
          theoretical: { days: 0, hours: 0, minutes: 6, seconds: 16, underASecond: false },
          efficiency: 90,
          connections: [
            {
              connection: null,
              rate: '100000000',
              span: { days: 0, hours: 0, minutes: 6, seconds: 58, underASecond: false },
            },
          ],
        };
      });
      const field = (id: string) => harness.element<HTMLInputElement>(`[data-testid="${id}"]`).value;

      harness.tool.sample();
      await vi.waitFor(() => expect(harness.tools.requestsOf('convert_size')).toHaveLength(1));
      await harness.settle();
      expect(harness.tool.result()?.title).toMatchObject({ key: 'tools.sizes.noteTitle' });

      await tab(harness, 'transfer');
      expect([field('transfer-size'), field('transfer-rate'), field('transfer-efficiency')]).toEqual([
        '4,7',
        '100',
        '90',
      ]);
      await vi.waitFor(() => expect(harness.tools.requestsOf('estimate_transfer')).toHaveLength(1));
      await harness.settle();
      expect(harness.tool.result()?.title).toMatchObject({ key: 'tools.sizes.noteTransfer' });

      harness.tool.clear();
      await harness.settle();
      expect([field('transfer-size'), field('transfer-rate')]).toEqual(['', '']);
    });
  });
});
