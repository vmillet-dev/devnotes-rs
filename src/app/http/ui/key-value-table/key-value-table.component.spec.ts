import { ComponentFixture, TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';
import { KeyValueRow } from '@core/model/http.model';
import { provideAppTesting } from '@testing/testing.providers';
import { KeyValueTableComponent } from './key-value-table.component';

const ROWS: readonly KeyValueRow[] = [
  { enabled: true, key: 'page', value: '1', description: 'Page' },
  { enabled: false, key: 'status', value: 'paid', description: '' },
];

describe('KeyValueTableComponent', () => {
  let fixture: ComponentFixture<KeyValueTableComponent>;
  let emitted: (readonly KeyValueRow[])[];

  beforeEach(async () => {
    TestBed.resetTestingModule();
    TestBed.configureTestingModule({ imports: [KeyValueTableComponent], providers: [provideAppTesting()] });
    fixture = TestBed.createComponent(KeyValueTableComponent);
    fixture.componentRef.setInput('rows', ROWS);
    fixture.componentRef.setInput('label', 'Paramètres');
    fixture.componentRef.setInput('kind', 'params');
    emitted = [];
    fixture.componentInstance.rowsChange.subscribe((rows) => emitted.push(rows));
    fixture.autoDetectChanges();
    await fixture.whenStable();
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
    expect(emitted[1]).toEqual([ROWS[1]]);
  });

  it('proposes names only when it is given some', async () => {
    expect(fixture.nativeElement.querySelector('datalist')).toBeNull();

    fixture.componentRef.setInput('suggestions', ['Accept']);
    await fixture.whenStable();
    expect(all('datalist option')).toHaveLength(1);
    expect(all('[data-testid="http-kv-key"]')[0]?.getAttribute('list')).toBe('http-params-names');
  });
});
