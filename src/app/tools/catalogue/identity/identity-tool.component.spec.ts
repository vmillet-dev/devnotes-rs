import { TestBed } from '@angular/core/testing';
import { By } from '@angular/platform-browser';
import { describe, expect, it, vi } from 'vitest';
import { IdentityAnswer } from '@core/model/tool-answers.model';
import { FILE_DIALOG_ADAPTER } from '@core/services/dialogs/file-dialog.service';
import { ChoiceMenuComponent } from '@shared/controls/choice-menu/choice-menu.component';
import { StatusNotifier } from '@core/services/notifications/status.service';
import { FakeFileDialog } from '@testing/fake-file-dialog';
import { FakeToolsRepository } from '@testing/fake-tools-repository';
import { ToolHarness, renderTool } from '@testing/tool-harness';
import { IdentityToolComponent, ageOn } from './identity-tool.component';

/** A person as Rust answers one: every number computed there, none here. */
const ADA: IdentityAnswer = {
  seed: '2676318943',
  json: '{\n  "surname": "Lefèvre"\n}',
  person: {
    country: 'FR',
    sex: 'F',
    surname: 'Lefèvre',
    givenNames: ['Élise', 'Marie'],
    birth: { date: '1987-03-12', city: 'Lyon', region: 'Rhône' },
    document: 'F7K2M9P4X',
    address: { street: '12 rue Pasteur', postcode: '44000', city: 'Nantes', region: 'Loire-Atlantique' },
    phone: '02 61 91 12 34',
    mobile: '06 39 98 56 78',
    email: 'elise.lefevre@example.org',
    bank: { name: 'LCL', iban: 'FR76 3000 6000 0112 3456 7890 189', bic: 'CRLYFRPP' },
    identifiers: [{ kind: 'nir', value: '2 87 03 69 123 456 07' }],
    company: {
      name: 'Lefèvre Conseil',
      identifiers: [
        { kind: 'siren', value: '404 833 048' },
        { kind: 'siret', value: '404 833 048 00022' },
        { kind: 'vat', value: 'FR83404833048' },
      ],
    },
  },
};

describe('IdentityToolComponent', () => {
  const answering = (tools: FakeToolsRepository): void => {
    tools.identity = ADA;
  };

  const requests = (harness: ToolHarness<IdentityToolComponent>) =>
    harness.tools.requestsOf('generate_identity');
  const text = (harness: ToolHarness<IdentityToolComponent>, testid: string) =>
    harness.element(`[data-testid="${testid}"]`)?.textContent?.replace(/\s+/g, ' ').trim();
  const rows = (harness: ToolHarness<IdentityToolComponent>, block: string) =>
    harness
      .all(`[data-testid="identity-block"][data-block="${block}"] [data-testid="output-row"]`)
      .map((row) => [row.dataset['name'], row.querySelector('[data-testid="output-value"]')?.textContent]);
  const drawn = async () => {
    const harness = await renderTool(IdentityToolComponent, answering);
    await vi.waitFor(() => expect(requests(harness)).toHaveLength(1));
    await harness.settle();
    return harness;
  };

  it('draws someone on opening, with no seed and the ages of an adult', async () => {
    const harness = await drawn();

    expect(requests(harness)).toEqual([{ country: 'FR', sex: null, ageMin: 18, ageMax: 80, seed: null }]);
    expect(text(harness, 'identity-seed-used')).toBe('· 2676318943 utilisée');
  });

  it('names the country in a menu, and keeps it when asked for one it does not have', async () => {
    const harness = await drawn();
    const menu = harness.fixture.debugElement.query(By.directive(ChoiceMenuComponent));

    expect(text(harness, 'choice-identity-country')).toContain('France');
    harness.element('[data-testid="choice-identity-country"]').click();
    await harness.settle();
    expect(harness.all('[data-testid="choice-option"]').map((option) => option.dataset['optionId'])).toEqual([
      'FR',
    ]);

    menu.triggerEventHandler('chosen', 'XX');
    menu.triggerEventHandler('chosen', 'FR');
    await harness.settle();
    expect(requests(harness).at(-1)).toMatchObject({ country: 'FR' });
  });

  /**
   * ⚠️ It evokes a card and imitates none: a monogram where a photo would be, and « Spécimen »
   * printed across — test data, not a document to pass off.
   */
  it('draws the person as a card: a monogram, the names large, and « Spécimen » across it', async () => {
    const harness = await drawn();
    const card = harness.element('[data-testid="identity-card"]');

    expect(card.dataset['country']).toBe('FR');
    expect(text(harness, 'identity-monogram')).toBe('ÉL');
    expect(text(harness, 'identity-surname')).toContain('Lefèvre');
    expect(text(harness, 'identity-given')).toContain('Élise, Marie');
    expect(harness.element('[data-testid="identity-sex"]').dataset['sex']).toBe('F');
    expect(text(harness, 'identity-sex')).toBe('Féminin');
    expect(card.textContent).toContain('Française');
    expect(harness.element('[data-testid="identity-birth"]').dataset['date']).toBe('1987-03-12');
    expect(text(harness, 'identity-birth')).toContain('12 mars 1987');
    expect(text(harness, 'identity-birth-place')).toBe('Lyon (Rhône)');
    expect(text(harness, 'identity-document')).toBe('F7K2M9P4X');
    // The number that is the person's own is on the card, under its own name.
    const personal = harness.element('[data-testid="identity-personal"]');
    expect(personal.dataset['kind']).toBe('nir');
    expect(personal.textContent).toContain('2 87 03 69 123 456 07');

    const specimen = harness.element('[data-testid="identity-specimen"]');
    expect(specimen.textContent?.trim()).toBe('Spécimen');
    expect(specimen.getAttribute('aria-hidden')).toBe('true');
    expect(card.contains(specimen)).toBe(true);
    // Nothing of a state's document: no image at all, a monogram alone.
    expect(card.querySelector('img, svg image')).toBeNull();
  });

  it('lays the rest in three blocks: the contact, the bank, the company', async () => {
    const harness = await drawn();

    expect(harness.all('[data-testid="identity-block"]').map((block) => block.dataset['block'])).toEqual([
      'contact',
      'bank',
      'company',
    ]);
    expect(rows(harness, 'contact')).toEqual([
      ['Adresse', '12 rue Pasteur, 44000 Nantes'],
      ['Téléphone', '02 61 91 12 34'],
      ['Mobile', '06 39 98 56 78'],
      ['E-mail', 'elise.lefevre@example.org'],
    ]);
    expect(rows(harness, 'bank')).toEqual([
      ['Banque', 'LCL'],
      ['IBAN', 'FR76 3000 6000 0112 3456 7890 189'],
      ['BIC', 'CRLYFRPP'],
    ]);
    expect(rows(harness, 'company')).toEqual([
      ['Raison sociale', 'Lefèvre Conseil'],
      ['SIREN', '404 833 048'],
      ['SIRET', '404 833 048 00022'],
      ['TVA intracommunautaire', 'FR83404833048'],
    ]);
  });

  it('copies each value with a click, from a block and from the card', async () => {
    const harness = await drawn();

    harness.element<HTMLButtonElement>('[data-field="iban"] [data-testid="copy-value"]').click();
    await expect.poll(() => harness.clipboard.content).toBe('FR76 3000 6000 0112 3456 7890 189');

    harness.element<HTMLButtonElement>('[data-testid="identity-surname"] [data-testid="copy-value"]').click();
    await expect.poll(() => harness.clipboard.content).toBe('Lefèvre');

    // The date is copied as a form takes it, not as the card writes it.
    harness.element<HTMLButtonElement>('[data-testid="identity-birth"] [data-testid="copy-value"]').click();
    await expect.poll(() => harness.clipboard.content).toBe('1987-03-12');
  });

  it('says what the data is for, and what in it could be someone’s', async () => {
    const harness = await drawn();
    const warning = harness.element('[data-testid="identity-warning"]');

    expect(warning.getAttribute('role')).toBe('note');
    expect(warning.textContent).toContain('ne les utilisez pas pour vous faire passer pour quelqu’un');
    expect(warning.textContent).toContain('peut appartenir à quelqu’un');
    expect(warning.textContent).toContain('givenNames');
  });

  it('asks for the sex and the ages chosen, held to what a person lives', async () => {
    const harness = await drawn();

    harness
      .element<HTMLButtonElement>('[data-testid="segmented-identity-sex"] [data-segment-id="M"]')
      .click();
    await vi.waitFor(() => expect(requests(harness).at(-1)).toMatchObject({ sex: 'M' }));
    harness
      .element<HTMLButtonElement>('[data-testid="segmented-identity-sex"] [data-segment-id="any"]')
      .click();
    await vi.waitFor(() => expect(requests(harness).at(-1)).toMatchObject({ sex: null }));

    await harness.type('identity-age-min', '30', 'generate_identity');
    await harness.type('identity-age-max', '500', 'generate_identity');
    expect(requests(harness).at(-1)).toMatchObject({ ageMin: 30, ageMax: 110 });

    // What is no number changes nothing.
    const before = requests(harness).length;
    const field = harness.element<HTMLInputElement>('[data-testid="identity-age-min"]');
    field.value = '';
    field.dispatchEvent(new Event('input'));
    await harness.settle();
    expect(requests(harness)).toHaveLength(before);
  });

  it('takes a seed that is typed, and pins the one it drew at its own field', async () => {
    const harness = await drawn();
    const pin = () => harness.element<HTMLButtonElement>('[data-testid="identity-pin-seed"]');
    expect(pin().getAttribute('aria-pressed')).toBe('false');
    expect(harness.element('[data-testid="identity-seed-pinned"]')).toBeNull();

    pin().click();
    await vi.waitFor(() => expect(requests(harness).at(-1)).toMatchObject({ seed: '2676318943' }));
    await harness.settle();
    expect(harness.element<HTMLInputElement>('[data-testid="identity-seed"]').value).toBe('2676318943');
    expect(pin().getAttribute('aria-pressed')).toBe('true');
    expect(text(harness, 'identity-seed-pinned')).toContain('la même personne revient');
    expect(harness.element('[data-testid="identity-seed-used"]')).toBeNull();

    pin().click();
    await vi.waitFor(() => expect(requests(harness).at(-1)).toMatchObject({ seed: null }));

    await harness.type('identity-seed', '  devnotes ', 'generate_identity');
    expect(requests(harness).at(-1)).toMatchObject({ seed: 'devnotes' });
    await harness.type('identity-seed', '   ', 'generate_identity');
    expect(requests(harness).at(-1)).toMatchObject({ seed: null });
  });

  it('draws someone else on « Régénérer », with the same options', async () => {
    const harness = await drawn();
    const action = harness.tool.actions()[0]!;
    expect(action).toMatchObject({ id: 'draw', labelKey: 'tools.identity.again', disabled: false });

    action.run();
    await vi.waitFor(() => expect(requests(harness)).toHaveLength(2));

    expect(requests(harness)[1]).toEqual(requests(harness)[0]);
  });

  it('copies the person as JSON, and keeps that JSON as a note', async () => {
    const harness = await drawn();

    harness.element<HTMLButtonElement>('[data-testid="identity-copy-json"]').click();
    await expect.poll(() => harness.clipboard.content).toBe(ADA.json);
    await vi.waitFor(() =>
      expect(TestBed.inject(StatusNotifier).status()?.key).toBe('tools.identity.jsonCopied'),
    );

    expect(harness.tool.result()).toEqual({
      title: { key: 'tools.identity.noteTitle', params: { name: 'Élise Lefèvre' } },
      kind: 'snippet',
      language: 'json',
      content: ADA.json,
    });
  });

  it('exports a batch to the file chosen, starting from the person on screen', async () => {
    const harness = await renderTool(IdentityToolComponent, (tools) => {
      tools.identity = ADA;
      tools.peopleExport = { kind: 'written', people: 25 };
    });
    await vi.waitFor(() => expect(requests(harness)).toHaveLength(1));
    await harness.settle();
    const dialog = TestBed.inject(FILE_DIALOG_ADAPTER) as FakeFileDialog;
    const exported = () => harness.tools.requestsOf('export_identities');

    // Nowhere chosen, nothing written.
    dialog.savePath = null;
    harness.element<HTMLButtonElement>('[data-testid="identity-export"]').click();
    await vi.waitFor(() => expect(dialog.saveCalls).toHaveLength(1));
    await harness.settle();
    expect(exported()).toEqual([]);
    expect(dialog.saveCalls[0]?.defaultPath).toBe('identities.json');

    dialog.savePath = 'C:/out/people.csv';
    const people = harness.element<HTMLInputElement>('[data-testid="identity-people"]');
    people.value = '25';
    people.dispatchEvent(new Event('input'));
    harness
      .element<HTMLButtonElement>('[data-testid="segmented-identity-format"] [data-segment-id="csv"]')
      .click();
    await harness.settle();
    harness.element<HTMLButtonElement>('[data-testid="identity-export"]').click();
    await vi.waitFor(() => expect(exported()).toHaveLength(1));

    expect(dialog.saveCalls[1]?.defaultPath).toBe('identities.csv');
    expect(exported()[0]).toEqual({
      // No seed was pinned: the one the card was drawn from is the file's.
      request: { country: 'FR', sex: null, ageMin: 18, ageMax: 80, seed: '2676318943' },
      count: 25,
      format: 'csv',
      path: 'C:/out/people.csv',
    });
    await vi.waitFor(() =>
      expect(TestBed.inject(StatusNotifier).status()).toMatchObject({
        key: 'tools.identity.exported',
        params: { count: 25 },
      }),
    );
  });

  it('holds a batch between one person and a hundred', async () => {
    const harness = await drawn();
    const people = harness.element<HTMLInputElement>('[data-testid="identity-people"]');
    const typed = async (value: string) => {
      people.value = value;
      people.dispatchEvent(new Event('input'));
      await harness.settle();
      return people.value;
    };

    expect(people.value).toBe('10');
    expect(await typed('5000')).toBe('100');
    expect(await typed('0')).toBe('1');
  });

  it('fills the sample’s seed, the same person for whoever opens the tool', async () => {
    const harness = await drawn();

    harness.tool.sample();
    await vi.waitFor(() => expect(requests(harness).at(-1)).toMatchObject({ seed: 'devnotes', sex: null }));
  });

  it('goes back to the first options on Vider, and draws someone new', async () => {
    const harness = await drawn();
    await harness.type('identity-seed', 'devnotes', 'generate_identity');
    await harness.type('identity-age-min', '40', 'generate_identity');
    const before = requests(harness).length;

    harness.tool.clear();
    await vi.waitFor(() => expect(requests(harness).length).toBeGreaterThan(before));

    expect(requests(harness).at(-1)).toEqual({
      country: 'FR',
      sex: null,
      ageMin: 18,
      ageMax: 80,
      seed: null,
    });
    await harness.settle();
    expect(harness.element<HTMLInputElement>('[data-testid="identity-seed"]').value).toBe('');
  });
});

describe('ageOn', () => {
  it('counts whole years, the birthday itself included', () => {
    expect(ageOn('1987-03-12', new Date(2026, 2, 11))).toBe(38);
    expect(ageOn('1987-03-12', new Date(2026, 2, 12))).toBe(39);
    expect(ageOn('1987-03-12', new Date(2026, 11, 31))).toBe(39);
    expect(ageOn('2026-10-12', new Date(2026, 9, 12))).toBe(0);
  });
});
