import { TestBed } from '@angular/core/testing';
import { describe, expect, it, vi } from 'vitest';
import { CheckAnswer } from '@core/model/tool-answers.model';
import { FakeToolsRepository } from '@testing/fake-tools-repository';
import { ToolHarness, renderTool } from '@testing/tool-harness';
import { ChecksToolComponent } from './checks-tool.component';

const VISA: CheckAnswer = {
  kind: 'luhn',
  grouped: '4111 1111 1111 1111',
  length: 16,
  valid: true,
  expectedLast: 1,
  completed: '4111 1111 1111 1111 7',
  network: 'visa',
  guessed: true,
};

const FRENCH_IBAN: CheckAnswer = {
  kind: 'iban',
  country: 'FR',
  printed: 'FR14 2004 1010 0505 0001 3M02 606',
  checkDigits: '14',
  length: 27,
  verdict: {
    kind: 'valid',
    bban: '20041010050500013M02606',
    bank: '20041',
    branch: '01005',
    account: '0500013M026',
    ribKey: { given: '06', expected: '06' },
  },
  guessed: true,
};

describe('ChecksToolComponent', () => {
  const answering =
    (answer: CheckAnswer) =>
    (tools: FakeToolsRepository): void => {
      tools.checks = answer;
    };

  const asked = (harness: ToolHarness<ChecksToolComponent>) => harness.tools.requestsOf('check_digits');
  const text = (harness: ToolHarness<ChecksToolComponent>, testid: string) =>
    harness.element(`[data-testid="${testid}"]`)?.textContent?.replace(/\s+/g, ' ').trim();
  const row = (harness: ToolHarness<ChecksToolComponent>, selector: string) =>
    harness.element(`${selector} [data-testid="output-value"]`)?.textContent?.replace(/\s+/g, ' ').trim();

  it('asks nothing of an empty field', async () => {
    const harness = await renderTool(ChecksToolComponent);

    expect(asked(harness)).toEqual([]);
    expect(harness.tool.result()).toBeNull();
  });

  it('checks a card number, names its network and completes it', async () => {
    const harness = await renderTool(ChecksToolComponent, answering(VISA));

    await harness.type('checks-input', '4111 1111 1111 1111', 'check_digits');

    expect(asked(harness)).toEqual([{ text: '4111 1111 1111 1111', kind: null }]);
    expect(harness.element('[data-testid="checks-verdict"]').dataset['valid']).toBe('true');
    expect(text(harness, 'checks-network')).toContain('Visa');
    expect(harness.element('[data-row="completed"] [data-testid="output-value"]').textContent).toBe(
      '4111 1111 1111 1111 7',
    );
    expect(text(harness, 'checks-reading')).toBe('Carte bancaire reconnue');
    expect(row(harness, '[data-row="length"]')).toBe('16 chiffres');
    expect(row(harness, '[data-row="key"]')).toBe('1 · clé de Luhn correcte');
  });

  it('says which last digit a wrong number asks for', async () => {
    const harness = await renderTool(
      ChecksToolComponent,
      answering({ ...VISA, grouped: '4111 1111 1111 1112', valid: false, network: null, guessed: false }),
    );

    await harness.type('checks-input', '4111 1111 1111 1112', 'check_digits');

    expect(text(harness, 'checks-verdict')).toContain('Le dernier chiffre devrait être 1.');
    expect(harness.element('[data-testid="checks-network"]')).toBeNull();
    expect(text(harness, 'checks-reading')).toBe('Numéro à clé de Luhn');
    expect(row(harness, '[data-row="key"]')).toBe('2 · 1 attendu');
  });

  it('checks an IBAN and names its country in the language on screen', async () => {
    const harness = await renderTool(ChecksToolComponent, answering(FRENCH_IBAN));

    await harness.type('checks-input', 'FR1420041010050500013M02606', 'check_digits');

    expect(harness.element('[data-testid="checks-reading"]').dataset['kind']).toBe('iban');
    expect(row(harness, '[data-testid="checks-country"]')).toBe('France (FR)');
    expect(row(harness, '[data-testid="checks-bank"]')).toBe('20041');
    expect(row(harness, '[data-row="branch"]')).toBe('01005');
    expect(row(harness, '[data-row="account"]')).toBe('0500013M026');
    expect(row(harness, '[data-testid="checks-rib"]')).toBe('06');
    expect(row(harness, '[data-row="length"]')).toBe('27 caractères, la longueur d’un IBAN FR');
    expect(row(harness, '[data-row="key"]')).toBe('14 · modulo 97 correct');
    expect(row(harness, '[data-row="compact"]')).toBe('FR1420041010050500013M02606');
    expect(harness.element('[data-row="bban"]')).toBeNull();
  });

  it('says a RIB key that is wrong though the IBAN adds up', async () => {
    const harness = await renderTool(ChecksToolComponent, (tools) => {
      tools.checks = {
        ...FRENCH_IBAN,
        verdict: { ...FRENCH_IBAN.verdict, ribKey: { given: '07', expected: '06' } } as never,
      };
    });

    await harness.type('checks-input', 'FR', 'check_digits');

    expect(harness.element('[data-testid="checks-rib"]').dataset['valid']).toBe('false');
    expect(row(harness, '[data-testid="checks-rib"]')).toBe('07 · 06 attendue');
  });

  it('gives the right check digits, the right length, or says what else is wrong', async () => {
    const harness = await renderTool(
      ChecksToolComponent,
      answering({
        ...FRENCH_IBAN,
        printed: 'FR15 2004 1010 0505 0001 3M02 606',
        checkDigits: '15',
        verdict: { kind: 'wrongChecksum', expected: '14', corrected: 'FR14 2004 1010 0505 0001 3M02 606' },
      }),
    );
    const verdict = () => harness.element('[data-testid="checks-verdict"]');

    await harness.type('checks-input', 'FR15', 'check_digits');
    expect(verdict().textContent).toContain('La clé devrait être 14.');
    expect(harness.element('[data-row="corrected"] [data-testid="output-value"]').textContent).toBe(
      'FR14 2004 1010 0505 0001 3M02 606',
    );

    expect(row(harness, '[data-row="key"]')).toBe('15 · 14 attendu');

    harness.tools.checks = { ...FRENCH_IBAN, verdict: { kind: 'wrongLength', expected: 27, found: 26 } };
    await harness.type('checks-input', 'FR wrong length', 'check_digits');
    expect(row(harness, '[data-row="length"]')).toBe('26 caractères, 27 attendus pour un IBAN FR');

    for (const [answer, problem] of [
      [{ kind: 'wrongLength', expected: 27, found: 26 }, 'wrongLength'],
      [{ kind: 'wrongFormat' }, 'wrongFormat'],
      [{ kind: 'unknownCountry' }, 'unknownCountry'],
    ] as const) {
      harness.tools.checks = { ...FRENCH_IBAN, verdict: answer };
      await harness.type('checks-input', `FR ${problem}`, 'check_digits');
      expect(verdict().dataset['problem']).toBe(problem);
    }
    expect(verdict().textContent).toContain('FR');
  });

  it('forces a kind, and lets the shape decide again', async () => {
    const harness = await renderTool(ChecksToolComponent, answering(VISA));
    await harness.type('checks-input', '12345678', 'check_digits');

    harness
      .element<HTMLButtonElement>('[data-testid="segmented-checks-kind"] [data-segment-id="iban"]')
      .click();
    await vi.waitFor(() => expect(asked(harness)).toHaveLength(2));
    harness
      .element<HTMLButtonElement>('[data-testid="segmented-checks-kind"] [data-segment-id="auto"]')
      .click();
    await vi.waitFor(() => expect(asked(harness)).toHaveLength(3));

    expect(asked(harness).map((request) => (request as { kind: string | null }).kind)).toEqual([
      null,
      'iban',
      null,
    ]);
  });

  it('says where a number stops being one', async () => {
    const harness = await renderTool(ChecksToolComponent, answering({ kind: 'unreadable', at: 11 }));

    await harness.type('checks-input', '4111 1111 x111', 'check_digits');
    expect(text(harness, 'checks-problem')).toContain('caractère 11');

    harness.tools.checks = { kind: 'tooShort' };
    await harness.type('checks-input', '7', 'check_digits');
    expect(harness.element('[data-testid="checks-problem"]').dataset['problem']).toBe('tooShort');
  });

  /** The banner is true, not decoration: nothing to save, so « Enregistrer comme note » is off. */
  it('offers nothing to save, a card number or an IBAN alike', async () => {
    const harness = await renderTool(ChecksToolComponent, answering(VISA));

    await harness.type('checks-input', '4111 1111 1111 1111', 'check_digits');
    expect(harness.tool.result()).toBeNull();

    harness.tools.checks = FRENCH_IBAN;
    await harness.type('checks-input', 'FR14', 'check_digits');
    expect(harness.tool.result()).toBeNull();
    expect(text(harness, 'checks-private')).toContain('l’enregistrement en note est désactivé');
  });

  it('never keeps the number for the session: a second opening finds the field empty', async () => {
    const harness = await renderTool(ChecksToolComponent, answering(VISA));
    await harness.type('checks-input', '4111 1111 1111 1111', 'check_digits');

    const again = TestBed.createComponent(ChecksToolComponent);
    again.autoDetectChanges();
    await again.whenStable();

    expect(again.nativeElement.querySelector('[data-testid="checks-input"]').value).toBe('');
  });

  it('empties the field on Vider', async () => {
    const harness = await renderTool(ChecksToolComponent, answering(VISA));
    await harness.type('checks-input', '4111 1111 1111 1111', 'check_digits');

    harness.tool.clear();
    await harness.settle();

    expect(harness.element<HTMLInputElement>('[data-testid="checks-input"]').value).toBe('');
  });
});
