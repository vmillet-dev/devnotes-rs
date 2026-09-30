import { TranslocoService } from '@jsverse/transloco';
import { describe, expect, it, vi } from 'vitest';
import { PercentResult, PercentagesAnswer } from '@core/model/tool-answers.model';
import { FakeToolsRepository } from '@testing/fake-tools-repository';
import { ToolHarness, renderTool } from '@testing/tool-harness';
import { PercentagesToolComponent } from './percentages-tool.component';

const answered = (x: string, y: string, exact: string, rounded = exact): PercentResult => ({
  kind: 'answered',
  x,
  y,
  exact,
  rounded,
});

const ALL: PercentagesAnswer = {
  of: answered('15', '240', '36'),
  share: answered('1', '3', '33.333333333333333333333333333', '33.33'),
  change: answered('80', '100', '25'),
  apply: answered('15', '2400', '2760'),
  before: answered('15', '276', '240'),
};

describe('PercentagesToolComponent', () => {
  const answering =
    (answer: PercentagesAnswer) =>
    (tools: FakeToolsRepository): void => {
      tools.percentages = answer;
    };

  const asked = (harness: ToolHarness<PercentagesToolComponent>) =>
    harness.tools.requestsOf('answer_percentages');
  const text = (harness: ToolHarness<PercentagesToolComponent>, testid: string) =>
    harness.element(`[data-testid="${testid}"]`)?.textContent?.trim();

  it('asks nothing while every field is empty', async () => {
    const harness = await renderTool(PercentagesToolComponent);

    expect(asked(harness)).toEqual([]);
    expect(harness.all('[data-testid="percentages-line"]').map((line) => line.dataset['question'])).toEqual([
      'of',
      'share',
      'change',
      'apply',
      'before',
    ]);
    expect(harness.tool.result()).toBeNull();
  });

  it('asks every question with what was typed, and answers each with its formula', async () => {
    const harness = await renderTool(PercentagesToolComponent, answering(ALL));

    await harness.type('percentages-of-x', '15', 'answer_percentages');
    await harness.type('percentages-of-y', '240', 'answer_percentages');

    expect(asked(harness).at(-1)).toEqual({
      of: { x: '15', y: '240' },
      share: { x: '', y: '' },
      change: { x: '', y: '' },
      apply: { x: '', y: '' },
      before: { x: '', y: '' },
      lower: false,
      decimals: 2,
    });
    expect(text(harness, 'percentages-of-result')).toBe('36');
    expect(text(harness, 'percentages-of-formula')).toBe('15 ÷ 100 × 240 = 36');
    expect(text(harness, 'percentages-share-result')).toBe('≈ 33,33 %');
    expect(text(harness, 'percentages-change-result')).toBe('+25 %');
    expect(text(harness, 'percentages-change-formula')).toBe('(100 − 80) ÷ |80| × 100 = +25 %');
    expect(text(harness, 'percentages-apply-result')).toBe('2 760');
    expect(text(harness, 'percentages-before-formula')).toBe('276 ÷ (1 + 15 ÷ 100) = 240');
  });

  it('lowers rather than raises once asked', async () => {
    const harness = await renderTool(PercentagesToolComponent, answering(ALL));
    await harness.type('percentages-apply-y', '2400', 'answer_percentages');

    harness
      .element<HTMLButtonElement>('[data-testid="segmented-percentages-direction"] [data-segment-id="lower"]')
      .click();
    await vi.waitFor(() => expect(asked(harness).at(-1)).toMatchObject({ lower: true }));
    await harness.settle();

    expect(text(harness, 'percentages-apply-formula')).toBe('2 400 × (1 − 15 ÷ 100) = 2 760');
  });

  it('writes a fall without a plus, and a nil change as zero', async () => {
    const harness = await renderTool(
      PercentagesToolComponent,
      answering({ ...ALL, change: answered('100', '80', '-20') }),
    );
    await harness.type('percentages-change-x', '100', 'answer_percentages');
    expect(text(harness, 'percentages-change-result')).toBe('-20 %');

    harness.tools.percentages = { ...ALL, change: answered('5', '5', '0') };
    await harness.type('percentages-change-y', '5', 'answer_percentages');
    expect(text(harness, 'percentages-change-result')).toBe('0 %');
  });

  it('copies the exact result', async () => {
    const harness = await renderTool(PercentagesToolComponent, answering(ALL));
    await harness.type('percentages-share-x', '1', 'answer_percentages');

    harness.element<HTMLButtonElement>('[data-question="share"] [data-testid="copy-value"]').click();

    await vi.waitFor(() => expect(harness.clipboard.content).toBe('33.333333333333333333333333333'));
  });

  it('says why a line has no answer', async () => {
    const harness = await renderTool(
      PercentagesToolComponent,
      answering({
        of: { kind: 'unreadable', field: 'y', at: 3 },
        share: { kind: 'divisionByZero' },
        change: { kind: 'fromZero' },
        apply: { kind: 'tooLarge' },
        before: { kind: 'empty' },
      }),
    );

    await harness.type('percentages-of-y', '24x', 'answer_percentages');

    expect(text(harness, 'percentages-of-problem')).toBe('Valeur : illisible à partir du caractère 3.');
    expect(harness.element('[data-testid="percentages-share-problem"]').dataset['problem']).toBe(
      'divisionByZero',
    );
    expect(harness.element('[data-testid="percentages-change-problem"]').dataset['problem']).toBe('fromZero');
    expect(harness.element('[data-testid="percentages-apply-problem"]').dataset['problem']).toBe('tooLarge');
    expect(harness.element('[data-testid="percentages-before-problem"]')).toBeNull();
    expect(harness.tool.result()).toBeNull();
  });

  it('asks with the decimals set, kept between 0 and 12', async () => {
    const harness = await renderTool(PercentagesToolComponent, answering(ALL));
    await harness.type('percentages-of-x', '15', 'answer_percentages');

    await harness.type('percentages-decimals', '4', 'answer_percentages');
    await harness.type('percentages-decimals', '-3', 'answer_percentages');
    const decimals = harness.element<HTMLInputElement>('[data-testid="percentages-decimals"]');
    decimals.value = '';
    decimals.dispatchEvent(new Event('input'));
    await harness.settle();

    expect(asked(harness).map((request) => (request as { decimals: number }).decimals)).toEqual([2, 4, 0]);
  });

  it('keeps every formula answered as a note, in the language on screen', async () => {
    const harness = await renderTool(
      PercentagesToolComponent,
      answering({ ...ALL, share: { kind: 'empty' } }),
    );
    await harness.type('percentages-of-x', '15', 'answer_percentages');

    harness.fixture.debugElement.injector.get(TranslocoService).setActiveLang('en');
    await harness.settle();

    expect(harness.tool.result()).toEqual({
      title: { key: 'tools.percentages.noteTitle' },
      kind: 'snippet',
      language: 'txt',
      content: [
        '15 ÷ 100 × 240 = 36',
        '(100 − 80) ÷ |80| × 100 = +25 %',
        '2,400 × (1 + 15 ÷ 100) = 2,760',
        '276 ÷ (1 + 15 ÷ 100) = 240',
      ].join('\n'),
    });
  });

  it('empties every field on Vider', async () => {
    const harness = await renderTool(PercentagesToolComponent, answering(ALL));
    await harness.type('percentages-of-x', '15', 'answer_percentages');

    harness.tool.clear();
    await harness.settle();

    expect(harness.element<HTMLInputElement>('[data-testid="percentages-of-x"]').value).toBe('');
    expect(harness.element('[data-testid="percentages-of-result"]')).toBeNull();
  });
});
