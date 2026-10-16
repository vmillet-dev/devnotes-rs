import { ComponentFixture, TestBed } from '@angular/core/testing';
import { By } from '@angular/platform-browser';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { provideTranslocoTesting } from '@testing/provide-transloco-testing';
import { Placeholder } from '@core/model/note.model';
import { PlaceholderFormComponent } from './placeholder-form.component';

const FIELDS: Placeholder[] = [
  { name: 'host', defaultValue: '', value: '' },
  { name: 'port', defaultValue: '5432', value: '' },
];

describe('PlaceholderFormComponent', () => {
  let fixture: ComponentFixture<PlaceholderFormComponent>;

  function inputs(): HTMLInputElement[] {
    return [...fixture.nativeElement.querySelectorAll('.field-input')];
  }

  async function type(index: number, value: string): Promise<void> {
    inputs()[index].value = value;
    inputs()[index].dispatchEvent(new Event('input'));
    await fixture.whenStable();
  }

  async function submit(): Promise<void> {
    fixture.debugElement.query(By.css('.fields-form')).triggerEventHandler('submit', new Event('submit'));
    await fixture.whenStable();
  }

  async function open(placeholders: readonly Placeholder[]): Promise<void> {
    fixture.componentRef.setInput('placeholders', placeholders);
    await fixture.whenStable();
  }

  beforeEach(async () => {
    TestBed.resetTestingModule();
    TestBed.configureTestingModule({
      imports: [PlaceholderFormComponent],
      providers: [provideTranslocoTesting()],
    });
    fixture = TestBed.createComponent(PlaceholderFormComponent);
    fixture.componentRef.setInput('placeholders', FIELDS);
    fixture.autoDetectChanges();
    await fixture.whenStable();
  });

  it('renders one row per field, named', () => {
    expect(
      [...fixture.nativeElement.querySelectorAll('.field-name')].map((el) =>
        (el as HTMLElement).textContent?.trim(),
      ),
    ).toEqual(['host', 'port']);
  });

  it('offers the values the note already holds', async () => {
    await open([{ name: 'host', defaultValue: '', value: 'db.internal' }]);

    expect(inputs().map((input) => input.value)).toEqual(['db.internal']);
  });

  it('leaves a default as a suggestion rather than a typed value', () => {
    expect(inputs().map((input) => input.value)).toEqual(['', '']);
    expect(inputs()[1].placeholder).toBe('5432');
  });

  it('submits every field, untouched ones included', async () => {
    let emitted: Record<string, string> | undefined;
    fixture.componentInstance.submitted.subscribe((values) => (emitted = values));
    await type(0, 'db.internal');

    await submit();

    expect(emitted).toEqual({ host: 'db.internal', port: '' });
  });

  it('lets a stored value be emptied on purpose', async () => {
    await open([{ name: 'host', defaultValue: '', value: 'db.internal' }]);
    let emitted: Record<string, string> | undefined;
    fixture.componentInstance.submitted.subscribe((values) => (emitted = values));

    await type(0, '');
    await submit();

    expect(emitted).toEqual({ host: '' });
  });

  it('offers a raw copy for a note that only looks templated', async () => {
    let emitted = 0;
    fixture.componentInstance.rawRequested.subscribe(() => (emitted += 1));

    const raw = [...fixture.nativeElement.querySelectorAll('.fields-action')].find((button) =>
      (button as HTMLElement).textContent?.includes('tel quel'),
    ) as HTMLButtonElement;
    raw.click();
    await fixture.whenStable();

    expect(emitted).toBe(1);
  });

  it('says how many values the snippet expects', () => {
    expect(fixture.nativeElement.querySelector('.fields-hint').textContent).toContain('2');
  });

  describe('opening', () => {
    // jsdom only tracks `document.activeElement` for attached elements.
    async function openAttached(placeholders: readonly Placeholder[]): Promise<void> {
      fixture = TestBed.createComponent(PlaceholderFormComponent);
      fixture.componentRef.setInput('placeholders', placeholders);
      document.body.appendChild(fixture.nativeElement);
      fixture.autoDetectChanges();
      await fixture.whenStable();
    }

    afterEach(() => fixture.nativeElement.remove());

    it('puts the focus in the first field, never on a button', async () => {
      await openAttached(FIELDS);

      expect(document.activeElement).toBe(inputs()[0]);
    });

    it('puts it in the first field still empty', async () => {
      await openAttached([
        { name: 'host', defaultValue: '', value: 'db.internal' },
        { name: 'port', defaultValue: '5432', value: '' },
      ]);

      expect(document.activeElement).toBe(inputs()[1]);
    });
  });
});
