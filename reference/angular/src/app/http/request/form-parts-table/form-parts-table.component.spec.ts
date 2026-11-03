import { ComponentFixture, TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { FormPartRow } from '@core/model/http.model';
import { FakeFileDialog } from '@testing/fake-file-dialog';
import { provideAppTesting } from '@testing/testing.providers';
import { FormPartsTableComponent } from './form-parts-table.component';

const PARTS: readonly FormPartRow[] = [
  { enabled: true, key: 'title', value: 'Facture', description: '', file: false },
  { enabled: true, key: 'scan', value: 'C:/scan.pdf', description: '', file: true },
];

describe('FormPartsTableComponent', () => {
  let fixture: ComponentFixture<FormPartsTableComponent>;
  let emitted: (readonly FormPartRow[])[];
  let fileDialog: FakeFileDialog;

  beforeEach(async () => {
    TestBed.resetTestingModule();
    fileDialog = new FakeFileDialog();
    TestBed.configureTestingModule({
      imports: [FormPartsTableComponent],
      providers: [provideAppTesting({ fileDialog })],
    });
    fixture = TestBed.createComponent(FormPartsTableComponent);
    fixture.componentRef.setInput('parts', PARTS);
    emitted = [];
    fixture.componentInstance.partsChange.subscribe((parts) => emitted.push(parts));
    fixture.autoDetectChanges();
    await fixture.whenStable();
  });

  const all = <E extends HTMLElement = HTMLElement>(selector: string): E[] => [
    ...fixture.nativeElement.querySelectorAll(selector),
  ];

  it('draws a text field as text and a file as its path, a blank row after them', () => {
    expect(all('[data-testid="http-part-row"]')).toHaveLength(3);
    expect(all<HTMLInputElement>('[data-testid="http-part-value"]')[0]?.value).toBe('Facture');
    expect(all('[data-testid="http-part-choose"]')[0]?.textContent?.trim()).toBe('C:/scan.pdf');
  });

  it('turns a text into a file and back, letting go of what was typed', () => {
    all<HTMLButtonElement>('[data-testid="http-part-kind"]')[0]!.click();
    expect(emitted[0]?.[0]).toEqual({ ...PARTS[0], file: true, value: '' });
  });

  it('takes a file chosen from the disk, and a row typed in the blank one', async () => {
    fileDialog.openPath = 'C:/autre.pdf';
    all<HTMLButtonElement>('[data-testid="http-part-choose"]')[0]!.click();
    await vi.waitFor(() => expect(emitted[0]?.[1]).toEqual({ ...PARTS[1], value: 'C:/autre.pdf' }));

    const key = all<HTMLInputElement>('[data-testid="http-part-key"]')[2]!;
    key.value = 'note';
    key.dispatchEvent(new Event('input'));
    expect(emitted[1]?.[2]).toEqual({ enabled: true, key: 'note', value: '', description: '', file: false });
  });

  it('sets a field aside from its box, and removes one', () => {
    const box = all<HTMLInputElement>('[data-testid="http-part-enabled"]')[0]!;
    box.checked = false;
    box.dispatchEvent(new Event('change'));
    all<HTMLButtonElement>('[data-testid="http-part-remove"]')[1]!.click();

    expect(emitted[0]?.[0]?.enabled).toBe(false);
    expect(emitted[1]).toEqual([PARTS[0]]);
  });
});
