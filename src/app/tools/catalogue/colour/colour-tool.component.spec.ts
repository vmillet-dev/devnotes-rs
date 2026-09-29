import { describe, expect, it, vi } from 'vitest';
import { ColourAnswer } from '@core/model/tool-answers.model';
import { FakeToolsRepository } from '@testing/fake-tools-repository';
import { ToolHarness, renderTool } from '@testing/tool-harness';
import { ColourToolComponent } from './colour-tool.component';

const DODGER: ColourAnswer = {
  colour: {
    kind: 'read',
    given: 'hex',
    notations: {
      hex: '#1e90ff',
      rgb: 'rgb(30 144 255)',
      hsl: 'hsl(209.6 100% 55.9%)',
      oklch: 'oklch(65.4% 0.186 253.2)',
    },
    swatch: '#1e90ff',
    outOfGamut: false,
  },
  against: {
    kind: 'read',
    given: 'hex',
    notations: { hex: '#ffffff', rgb: 'rgb(255 255 255)', hsl: 'hsl(0 0% 100%)', oklch: 'oklch(100% 0 0)' },
    swatch: '#ffffff',
    outOfGamut: false,
  },
  contrast: { ratio: 3.24, aa: { normal: false, large: true }, aaa: { normal: false, large: false } },
};

describe('ColourToolComponent', () => {
  const answer = (tools: FakeToolsRepository): void => {
    tools.colour = DODGER;
  };

  async function rendered(prepare = answer): Promise<ToolHarness<ColourToolComponent>> {
    const harness = await renderTool(ColourToolComponent, prepare);
    await vi.waitFor(() => expect(harness.tools.requestsOf('describe_colour')).toHaveLength(1));
    await harness.settle();
    return harness;
  }

  it('describes the colour it opens on, against white', async () => {
    const harness = await rendered();

    expect(harness.tools.requestsOf('describe_colour')[0]).toEqual({ colour: '#1e90ff', against: '#ffffff' });
    expect(harness.all('[data-testid="output-value"]').map((value) => value.textContent)).toEqual([
      '#1e90ff',
      'rgb(30 144 255)',
      'hsl(209.6 100% 55.9%)',
      'oklch(65.4% 0.186 253.2)',
    ]);
    expect(harness.element('[data-testid="colour-swatch"]').dataset['swatch']).toBe('#1e90ff');
  });

  it('asks again for what is typed, on either side', async () => {
    const harness = await rendered();

    await harness.type('colour-input', 'oklch(70% 0.25 150)', 'describe_colour');
    await harness.type('colour-against', '#15171c', 'describe_colour');

    expect(harness.tools.requestsOf('describe_colour').at(-1)).toEqual({
      colour: 'oklch(70% 0.25 150)',
      against: '#15171c',
    });
  });

  it('takes a colour from the system picker as if it had been typed', async () => {
    const harness = await rendered();
    const picker = harness.element<HTMLInputElement>('[data-testid="colour-against-picker"]');

    expect(picker.value).toBe('#ffffff');
    picker.value = '#15171c';
    picker.dispatchEvent(new Event('input'));
    await vi.waitFor(() => expect(harness.tools.requestsOf('describe_colour')).toHaveLength(2));

    expect(harness.tools.requestsOf('describe_colour')[1]).toEqual({ colour: '#1e90ff', against: '#15171c' });
    expect(harness.element<HTMLInputElement>('[data-testid="colour-against"]').value).toBe('#15171c');
  });

  it('empties both colours on Vider', async () => {
    const harness = await rendered();

    harness.tool.clear();
    await vi.waitFor(() => expect(harness.tools.requestsOf('describe_colour')).toHaveLength(2));

    expect(harness.tools.requestsOf('describe_colour')[1]).toEqual({ colour: '', against: '' });
    expect(harness.element<HTMLInputElement>('[data-testid="colour-input"]').value).toBe('');
  });

  it('gives the ratio and the WCAG verdicts', async () => {
    const harness = await rendered();

    expect(harness.element('[data-testid="colour-ratio"]').textContent).toBe('3.24:1');
    const cells = harness
      .all('[data-testid="colour-verdicts"] td')
      .map((cell) => cell.classList.contains('pass'));
    expect(cells).toEqual([false, true, false, false]);
  });

  it('says a colour sRGB cannot show was brought inside', async () => {
    const harness = await rendered((tools) => {
      tools.colour = { ...DODGER, colour: { ...DODGER.colour, outOfGamut: true } as ColourAnswer['colour'] };
    });

    expect(harness.element('[data-testid="colour-gamut"]')).not.toBeNull();
  });

  it('says what is not a colour', async () => {
    const harness = await rendered((tools) => {
      tools.colour = { colour: { kind: 'invalid' }, against: { kind: 'empty' }, contrast: null };
    });

    expect(harness.element('[data-testid="colour-invalid"]')).not.toBeNull();
    expect(harness.tool.result()).toBeNull();
  });

  it('keeps the four notations as CSS', async () => {
    const harness = await rendered();

    expect(harness.tool.result()).toEqual({
      title: { key: 'tools.colour.noteTitle', params: { hex: '#1e90ff' } },
      kind: 'snippet',
      language: 'css',
      content: '#1e90ff\nrgb(30 144 255)\nhsl(209.6 100% 55.9%)\noklch(65.4% 0.186 253.2)',
    });
  });
});
