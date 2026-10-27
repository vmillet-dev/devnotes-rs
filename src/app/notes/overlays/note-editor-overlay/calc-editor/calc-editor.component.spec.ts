import { ComponentFixture, TestBed } from '@angular/core/testing';
import { TranslocoService } from '@jsverse/transloco';
import { firstValueFrom } from 'rxjs';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { startCompletion } from '@codemirror/autocomplete';
import { EditorView } from '@codemirror/view';
import type { CalcRow, CalcSheet, CalcValue, CalcWordEntry } from '@core/model/calc.model';
import { CALC_WORD_TABLE } from '@core/services/calc/calc-words';
import { SettingsStore } from '@core/services/settings/settings.store';
import { FakeCalcRepository } from '@testing/fake-calc-repository';
import { FakeClipboard } from '@testing/fake-clipboard';
import { provideAppTesting } from '@testing/testing.providers';
import { CalcEditorComponent } from './calc-editor.component';

const euros = (value: string): CalcValue => ({ kind: 'number', value, unit: { unit: 'euro', per: null } });

function value(of: CalcValue, name: string | null = null, aggregate = false): CalcRow {
  return {
    line: { kind: 'value', value: of, name, aggregate, dimension: 'euro', places: null },
    spans: name ? [{ start: 0, end: name.length, kind: 'name' }] : [],
    converting: null,
  };
}

/** The editor knows the words a sheet reads from this table alone. */
const WORDS: readonly CalcWordEntry[] = [
  { word: 'convert', class: 'syntax', fr: ['en'], en: ['in'] },
  { word: 'tomorrow', class: 'value', fr: ['demain'], en: ['tomorrow'] },
  { word: 'hex', class: 'value', fr: ['hex'], en: ['hex'] },
];

const TEXT = '# Budget\nhébergement = 100,521 €\ndomaines = 4,99 €\nmensuel = total\ndistance + 3 Go';
const SHEET: CalcSheet = {
  rows: [
    {
      line: { kind: 'heading', title: 'Budget' },
      spans: [{ start: 0, end: 8, kind: 'heading' }],
      converting: null,
    },
    value(euros('100.521'), 'hébergement'),
    value(euros('4.99'), 'domaines'),
    value(euros('105.511'), 'mensuel', true),
    {
      line: { kind: 'error', error: { kind: 'unknownName', name: 'distance' } },
      spans: [{ start: 0, end: 8, kind: 'error' }],
      converting: null,
    },
  ],
  variables: [{ name: 'hébergement', line: 1, value: euros('100.521'), uses: [3] }],
};

describe('CalcEditorComponent', () => {
  let fixture: ComponentFixture<CalcEditorComponent>;
  let calc: FakeCalcRepository;
  let clipboard: FakeClipboard;
  let changes: string[];

  const view = (): EditorView => EditorView.findFromDOM(fixture.nativeElement.querySelector('.cm-editor'))!;
  const results = (): HTMLElement[] => [
    ...fixture.nativeElement.querySelectorAll('[data-testid="calc-result"]'),
  ];
  const text = (selector: string): string =>
    (fixture.nativeElement.querySelector(selector)?.textContent ?? '')
      .replace(/[\s\u00a0\u202f]+/g, ' ')
      .trim();

  /** A press, then the click that acts, as a pointer gives them. */
  function mouse(target: Element, init: MouseEventInit = {}): void {
    target.dispatchEvent(new MouseEvent('mousedown', { bubbles: true, cancelable: true, ...init }));
    target.dispatchEvent(new MouseEvent('click', { bubbles: true, cancelable: true, ...init }));
  }

  beforeEach(async () => {
    TestBed.resetTestingModule();
    calc = new FakeCalcRepository();
    calc.sheet = SHEET;
    clipboard = new FakeClipboard();
    TestBed.configureTestingModule({
      imports: [CalcEditorComponent],
      providers: [
        provideAppTesting({ calcRepository: calc, clipboard }),
        { provide: CALC_WORD_TABLE, useValue: WORDS },
      ],
    });
    await firstValueFrom(TestBed.inject(TranslocoService).load('fr'));
    fixture = TestBed.createComponent(CalcEditorComponent);
    fixture.componentRef.setInput('session', 1);
    fixture.componentRef.setInput('content', TEXT);
    changes = [];
    fixture.componentInstance.changed.subscribe((next) => changes.push(next));
    document.body.appendChild(fixture.nativeElement);
    fixture.autoDetectChanges();
    await fixture.whenStable();
    await vi.waitFor(() => expect(results()).toHaveLength(4));
  });

  afterEach(() => fixture.nativeElement.remove());

  it('asks Rust for the whole sheet on the local day, and draws a result at the end of each line', () => {
    expect(calc.asked[0]).toEqual({ sheet: TEXT, today: expect.stringMatching(/^\d{4}-\d{2}-\d{2}$/) });
    expect(results().map((result) => result.textContent?.replace(/[\u00a0\u202f]/g, ' '))).toEqual([
      '≈ 100,52 €',
      '4,99 €',
      '≈ 105,51 €',
      '« distance » n’est défini nulle part au-dessus',
    ]);
    expect(results()[0]!.classList.contains('calc-named')).toBe(true);
    expect(results()[3]!.classList.contains('calc-failed')).toBe(true);
  });

  it('draws a heading, a total under a rule and the words in their colours', () => {
    const lines = [...fixture.nativeElement.querySelectorAll('.cm-line')] as HTMLElement[];
    expect(lines[0]!.classList.contains('calc-heading')).toBe(true);
    expect(lines[3]!.classList.contains('calc-total')).toBe(true);
    expect(lines[1]!.querySelector('.calc-name')?.textContent).toBe('hébergement');
  });

  it('counts the lines and the variables in its footer', () => {
    expect(text('.counts')).toBe('5 lignes · 1 variable');
  });

  it('adds up the results picked, Ctrl for one more, and says when there is nothing to add', async () => {
    mouse(results()[0]!.querySelector('[data-pick]')!);
    await fixture.whenStable();
    mouse(results()[1]!.querySelector('[data-pick]')!, { ctrlKey: true });
    await fixture.whenStable();

    expect(text('[data-testid="calc-summary"]')).toBe(
      '2 lignes sélectionnées · somme ≈ 105,51 € · moyenne ≈ 52,76 €',
    );
    await vi.waitFor(() =>
      expect(results().filter((result) => result.classList.contains('calc-selected'))).toHaveLength(2),
    );

    view().dispatch({ selection: { anchor: 0, head: 8 } });
    await fixture.whenStable();
    expect(text('[data-testid="calc-summary"]')).toBe('1 ligne sélectionnée · rien à additionner');
  });

  it('copies one result with every decimal, and says so in the footer', async () => {
    mouse(results()[0]!.querySelector('[data-copy]')!);

    await vi.waitFor(() => expect(clipboard.content.replace(/\u00a0/g, ' ')).toBe('100,521 €'));
    await vi.waitFor(() => expect(text('[data-testid="calc-summary"]')).toBe('Copié : 100,521 €'));
  });

  it('keeps the decimals chosen as a preference, and draws the results again with them', async () => {
    const select: HTMLSelectElement = fixture.nativeElement.querySelector('[data-testid="calc-decimals"]');
    select.value = '0';
    select.dispatchEvent(new Event('change'));
    await fixture.whenStable();

    expect(TestBed.inject(SettingsStore).calcDecimals()).toBe('0');
    await vi.waitFor(() => expect(results()[1]!.textContent?.replace(/\u00a0/g, ' ')).toBe('≈ 5 €'));
  });

  it('proposes after « en » only the units of the kind Rust read on the line, in the language on screen', async () => {
    calc.sheet = {
      rows: [
        {
          line: { kind: 'error', error: { kind: 'unreadable', at: 12 } },
          spans: [],
          converting: 'data',
        },
      ],
      variables: [],
    };
    const typed = '1,5 Go en M';
    view().dispatch({
      changes: { from: 0, to: view().state.doc.length, insert: typed },
      selection: { anchor: typed.length },
    });
    startCompletion(view());

    await vi.waitFor(() =>
      expect(
        [...document.querySelectorAll('.cm-completionLabel')].map((label) => label.textContent),
      ).toContain('Mo'),
    );
    const labels = [...document.querySelectorAll('.cm-completionLabel')].map((label) => label.textContent);
    expect(labels).toContain('Mio');
    expect(labels).not.toContain('min');
  });

  it('proposes after « en » on a plain number a percentage and the bases of the table', async () => {
    calc.sheet = {
      rows: [
        { line: { kind: 'error', error: { kind: 'unreadable', at: 7 } }, spans: [], converting: 'number' },
      ],
      variables: [],
    };
    const typed = '255 en ';
    view().dispatch({
      changes: { from: 0, to: view().state.doc.length, insert: typed },
      selection: { anchor: typed.length },
    });
    startCompletion(view());

    await vi.waitFor(() =>
      expect([...document.querySelectorAll('.cm-completionLabel')].map((label) => label.textContent)).toEqual(
        ['%', 'hex'],
      ),
    );
  });

  it('proposes a word of the table in the spelling of the language on screen', async () => {
    const labels = (): (string | null)[] =>
      [...document.querySelectorAll('.cm-completionLabel')].map((label) => label.textContent);
    const type = (typed: string): void => {
      view().dispatch({
        changes: { from: 0, to: view().state.doc.length, insert: typed },
        selection: { anchor: typed.length },
      });
      startCompletion(view());
    };

    type('dem');
    await vi.waitFor(() => expect(labels()).toEqual(['demain']));

    type('tom');
    await vi.waitFor(() => expect(labels()).toEqual([]));
    type('// dem');
    await fixture.whenStable();
    expect(labels()).toEqual([]);
  });

  it('marks a failed line with the reason Rust gave, its message in its place', () => {
    expect(results()[3]!.dataset['error']).toBe('unknownName');
    expect(results()[0]!.dataset['error']).toBeUndefined();
  });

  it('hands every edit out and asks Rust again once the typing pauses', async () => {
    const before = calc.asked.length;
    view().dispatch({
      changes: { from: view().state.doc.length, insert: '\n2 + 2' },
      userEvent: 'input.type',
    });

    expect(changes.at(-1)).toBe(`${TEXT}\n2 + 2`);
    await vi.waitFor(() => expect(calc.asked.length).toBeGreaterThan(before));
    expect(calc.asked.at(-1)?.sheet).toBe(`${TEXT}\n2 + 2`);
  });
});
