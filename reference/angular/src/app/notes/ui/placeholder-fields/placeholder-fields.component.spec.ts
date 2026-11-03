import { ComponentFixture, TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';
import { provideTranslocoTesting } from '@testing/provide-transloco-testing';
import { PlaceholderFieldsComponent, PlaceholderValue } from './placeholder-fields.component';

describe('PlaceholderFieldsComponent', () => {
  let fixture: ComponentFixture<PlaceholderFieldsComponent>;

  function inputs(): HTMLInputElement[] {
    return [...fixture.nativeElement.querySelectorAll('.field-input')];
  }

  beforeEach(async () => {
    TestBed.resetTestingModule();
    TestBed.configureTestingModule({
      imports: [PlaceholderFieldsComponent],
      providers: [provideTranslocoTesting()],
    });
    fixture = TestBed.createComponent(PlaceholderFieldsComponent);
    fixture.componentRef.setInput('placeholders', [
      { name: 'host', defaultValue: '', value: '' },
      { name: 'port', defaultValue: '5432', value: '' },
    ]);
    fixture.componentRef.setInput('values', { host: 'db.internal' });
    fixture.autoDetectChanges();
    await fixture.whenStable();
  });

  it('shows the value it is given, and nothing for a field without one', () => {
    expect(inputs().map((input) => input.value)).toEqual(['db.internal', '']);
  });

  it('offers the default as a suggestion rather than as a value', () => {
    expect(inputs()[1].placeholder).toBe('5432');
    expect(inputs()[1].value).toBe('');
  });

  it('emits the field that changed, named', async () => {
    const emitted: PlaceholderValue[] = [];
    fixture.componentInstance.changed.subscribe((value) => emitted.push(value));

    inputs()[0].value = 'db.staging';
    inputs()[0].dispatchEvent(new Event('input'));
    await fixture.whenStable();

    expect(emitted).toEqual([{ name: 'host', value: 'db.staging' }]);
  });

  it('labels each input with the field it fills', () => {
    expect(inputs().map((input) => input.getAttribute('aria-label'))).toEqual([
      'Valeur de host',
      'Valeur de port',
    ]);
  });
});
