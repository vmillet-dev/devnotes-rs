import { ComponentFixture, TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { KeyValueRow } from '@core/model/http.model';
import { FakeHttpRepository } from '@testing/fake-http-repository';
import { provideAppTesting } from '@testing/testing.providers';
import { KeyValueTableComponent } from './key-value-table.component';

const ROWS: readonly KeyValueRow[] = [
  { enabled: true, key: 'page', value: '1', description: 'Page' },
  { enabled: false, key: 'status', value: 'paid', description: '' },
];

describe('KeyValueTableComponent', () => {
  let fixture: ComponentFixture<KeyValueTableComponent>;
  let emitted: (readonly KeyValueRow[])[];
  let http: FakeHttpRepository;

  /**
   * As its host would: what the table emits comes back as its rows — `answering` unset, not
   * yet, which is a host whose screen has not caught up with what was typed.
   */
  async function render(rows: readonly KeyValueRow[], headers = false, answering = true) {
    TestBed.resetTestingModule();
    TestBed.configureTestingModule({
      imports: [KeyValueTableComponent],
      providers: [provideAppTesting({ httpRepository: http })],
    });
    fixture = TestBed.createComponent(KeyValueTableComponent);
    fixture.componentRef.setInput('rows', rows);
    fixture.componentRef.setInput('label', headers ? 'En-têtes' : 'Paramètres');
    fixture.componentRef.setInput('kind', headers ? 'headers' : 'params');
    fixture.componentRef.setInput('headers', headers);
    emitted = [];
    fixture.componentInstance.rowsChange.subscribe((next) => {
      emitted.push(next);
      if (answering) fixture.componentRef.setInput('rows', next);
    });
    fixture.autoDetectChanges();
    await fixture.whenStable();
  }

  beforeEach(async () => {
    http = new FakeHttpRepository();
    await render(ROWS);
  });

  const all = <E extends HTMLElement = HTMLElement>(selector: string): E[] => [
    ...fixture.nativeElement.querySelectorAll(selector),
  ];
  const type = (field: HTMLInputElement, text: string) => {
    field.value = text;
    field.dispatchEvent(new Event('input'));
  };

  it('draws each row, a row not sent set apart, and a blank row to add one', () => {
    expect(all('[data-testid="http-kv-row"]')).toHaveLength(3);
    expect(all('[data-testid="http-kv-row"]')[1]?.classList).toContain('off');
    expect(all<HTMLInputElement>('[data-testid="http-kv-enabled"]').map((box) => box.checked)).toEqual([
      true,
      false,
    ]);
    expect(all('[data-testid="http-kv-remove"]')).toHaveLength(2);
  });

  it('turns typing in the blank row into a row, and edits the others in place', () => {
    type(all<HTMLInputElement>('[data-testid="http-kv-key"]')[2]!, 'limit');
    type(all<HTMLInputElement>('[data-testid="http-kv-value"]')[0]!, '2');

    expect(emitted[0]).toEqual([...ROWS, { enabled: true, key: 'limit', value: '', description: '' }]);
    expect(emitted[1]?.[0]).toEqual({ enabled: true, key: 'page', value: '2', description: 'Page' });
  });

  it('sends or sets aside a row from its box, and removes one', () => {
    const box = all<HTMLInputElement>('[data-testid="http-kv-enabled"]')[1]!;
    box.checked = true;
    box.dispatchEvent(new Event('change'));
    all<HTMLButtonElement>('[data-testid="http-kv-remove"]')[0]!.click();

    expect(emitted[0]?.[1]?.enabled).toBe(true);
    expect(emitted[1]).toEqual([{ ...ROWS[1], enabled: true }]);
  });

  /** A parameter or a form field takes any character: the rule is the headers' alone. */
  it('asks Rust nothing about parameters, and splits nothing typed in their names', async () => {
    type(all<HTMLInputElement>('[data-testid="http-kv-key"]')[0]!, 'filter:status');
    await fixture.whenStable();

    expect(http.callsOf('checkHeaders')).toEqual([]);
    expect(http.callsOf('readHeaders')).toEqual([]);
    expect(emitted.at(-1)?.[0]).toMatchObject({ key: 'filter:status', value: '1' });
    expect(all('[data-testid="http-kv-fault"]')).toEqual([]);
  });

  /**
   * Key `X-Recette: 1` in a folder's headers was stored as typed, and every request under the
   * folder failed on its URL. The row says it where it is typed, by what Rust answers.
   */
  it('marks the header Rust says cannot be sent, and says why under the table', async () => {
    http.headerFaults = [null, 'name', 'value'];
    const rows = [
      { enabled: true, key: 'Accept', value: '*/*', description: '' },
      { enabled: true, key: 'X Recette', value: '1', description: '' },
      { enabled: true, key: 'X-Trace', value: 'a', description: '' },
    ];
    await render(rows, true);

    expect(http.callsOf('checkHeaders').at(-1)).toEqual([rows]);
    expect(all('[data-testid="http-kv-row"]').map((row) => row.dataset['fault'] ?? null)).toEqual([
      null,
      'name',
      'value',
      null,
    ]);
    const names = all<HTMLInputElement>('[data-testid="http-kv-key"]');
    const values = all<HTMLInputElement>('[data-testid="http-kv-value"]');
    expect(names.map((cell) => cell.getAttribute('aria-invalid'))).toEqual([null, 'true', null, null]);
    expect(values.map((cell) => cell.getAttribute('aria-invalid'))).toEqual([null, null, 'true', null]);
    const said = all('[data-testid="http-kv-fault"]');
    expect(said.map((line) => line.textContent?.trim())).toEqual([
      "Un nom d'en-tête ne contient ni espace, ni deux-points, ni accent : la ligne marquée ne peut pas être envoyée.",
      "Une valeur d'en-tête tient sur une ligne, sans caractère de contrôle : la ligne marquée ne peut pas être envoyée.",
    ]);
    // The cell names the sentence that explains it.
    expect(names[1]!.getAttribute('aria-describedby')).toBe(said[0]!.id);
    expect(values[2]!.getAttribute('aria-describedby')).toBe(said[1]!.id);

    http.headerFaults = [null, null, null];
    type(names[1]!, 'X-Recette');
    await vi.waitFor(() => expect(all('[data-testid="http-kv-fault"]')).toEqual([]));
    expect(all('[data-testid="http-kv-row"]').some((row) => row.dataset['fault'])).toBe(false);
  });

  it('splits « Nom: valeur » typed in a name into its two cells, and moves on to the value', async () => {
    await render([], true);
    http.headersRead = [{ enabled: true, key: 'X-Recette', value: '1', description: '' }];
    const name = all<HTMLInputElement>('[data-testid="http-kv-key"]')[0]!;

    type(name, 'X-Recette: 1');
    await vi.waitFor(() =>
      expect(emitted.at(-1)).toEqual([{ enabled: true, key: 'X-Recette', value: '1', description: '' }]),
    );

    expect(http.callsOf('readHeaders')).toEqual([['X-Recette: 1']]);
    await fixture.whenStable();
    expect(document.activeElement).toBe(all('[data-testid="http-kv-value"]')[0]);
  });

  /**
   * ⚠️ `rows` follows what was emitted only at the next change detection. Starting from it, a
   * value typed right after the split put the colon back in the name, and an answer from Rust
   * arriving first found no row to move.
   */
  it('starts from what it last emitted, the host not having answered yet', async () => {
    await render([], true, false);
    http.headersRead = [{ enabled: true, key: 'X-Recette', value: '', description: '' }];

    type(all<HTMLInputElement>('[data-testid="http-kv-key"]')[0]!, 'X-Recette:');
    await vi.waitFor(() => expect(emitted).toHaveLength(2));
    type(all<HTMLInputElement>('[data-testid="http-kv-value"]')[0]!, ' 1');

    expect(emitted.at(-1)).toEqual([{ enabled: true, key: 'X-Recette', value: ' 1', description: '' }]);

    type(all<HTMLInputElement>('[data-testid="http-kv-key"]')[0]!, 'X-Autre');
    expect(emitted.at(-1)).toEqual([{ enabled: true, key: 'X-Autre', value: ' 1', description: '' }]);
  });

  it('moves nothing when the row has a value already, or when Rust reads no header in it', async () => {
    await render([{ enabled: true, key: 'X-A', value: 'kept', description: 'd' }], true);
    http.headersRead = [{ enabled: true, key: 'X-A', value: '2', description: '' }];
    type(all<HTMLInputElement>('[data-testid="http-kv-key"]')[0]!, 'X-A: 2');
    await vi.waitFor(() => expect(http.callsOf('readHeaders')).toHaveLength(1));
    await fixture.whenStable();
    expect(emitted.at(-1)).toEqual([{ enabled: true, key: 'X-A: 2', value: 'kept', description: 'd' }]);

    http.headersRead = [];
    type(all<HTMLInputElement>('[data-testid="http-kv-key"]')[1]!, ':authority');
    await vi.waitFor(() => expect(http.callsOf('readHeaders')).toHaveLength(2));
    await fixture.whenStable();
    expect(emitted.at(-1)?.[1]).toMatchObject({ key: ':authority', value: '' });
  });

  /** A browser's network tab copies headers a line each: the field alone would take one line. */
  it('fills a row a line when several are pasted in a name, after a row already written', async () => {
    const paste = (cell: HTMLInputElement, text: string) => {
      const event = new Event('paste', { cancelable: true }) as ClipboardEvent;
      Object.defineProperty(event, 'clipboardData', { value: { getData: () => text } });
      cell.dispatchEvent(event);
      return event;
    };
    const read = [
      { enabled: true, key: 'accept', value: '*/*', description: '' },
      { enabled: true, key: 'x-client', value: 'web', description: '' },
    ];
    await render([], true);
    http.headersRead = read;

    const pasted = paste(
      all<HTMLInputElement>('[data-testid="http-kv-key"]')[0]!,
      'accept: */*\nx-client: web\n',
    );
    expect(pasted.defaultPrevented).toBe(true);
    await vi.waitFor(() => expect(emitted.at(-1)).toEqual(read));
    expect(http.callsOf('readHeaders')).toEqual([['accept: */*\nx-client: web\n']]);

    // In a row already written, they go after it.
    const written = { enabled: true, key: 'Accept', value: 'text/csv', description: '' };
    await render([written], true);
    http.headersRead = read;
    paste(all<HTMLInputElement>('[data-testid="http-kv-key"]')[0]!, 'accept: */*\nx-client: web');
    await vi.waitFor(() => expect(emitted.at(-1)).toEqual([written, ...read]));

    // One line is the field's own to take, and lines that hold no header change nothing.
    expect(paste(all<HTMLInputElement>('[data-testid="http-kv-key"]')[0]!, 'X-A: 1\n').defaultPrevented).toBe(
      false,
    );
    http.headersRead = [];
    const before = emitted.length;
    paste(all<HTMLInputElement>('[data-testid="http-kv-key"]')[0]!, 'GET / HTTP/2\nnothing');
    await fixture.whenStable();
    expect(emitted).toHaveLength(before);
  });

  it('leaves a paste in a parameter name to the field', async () => {
    const event = new Event('paste', { cancelable: true }) as ClipboardEvent;
    Object.defineProperty(event, 'clipboardData', { value: { getData: () => 'a: 1\nb: 2' } });
    all<HTMLInputElement>('[data-testid="http-kv-key"]')[0]!.dispatchEvent(event);
    await fixture.whenStable();

    expect(event.defaultPrevented).toBe(false);
    expect(http.callsOf('readHeaders')).toEqual([]);
  });

  it('proposes names only when it is given some', async () => {
    expect(fixture.nativeElement.querySelector('datalist')).toBeNull();

    fixture.componentRef.setInput('suggestions', ['Accept']);
    await fixture.whenStable();
    expect(all('datalist option')).toHaveLength(1);
    expect(all('[data-testid="http-kv-key"]')[0]?.getAttribute('list')).toBe('http-params-names');
  });
});
