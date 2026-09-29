import { ComponentFixture, TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';
import { DEFAULT_PRETTIER_SETTINGS, PrettierSettings } from '@core/services/format/format.model';
import { provideTranslocoTesting } from '@testing/provide-transloco-testing';
import { FormatButtonComponent } from './format-button.component';

describe('FormatButtonComponent', () => {
  let fixture: ComponentFixture<FormatButtonComponent>;
  let formats: number;
  let changes: Partial<PrettierSettings>[];

  beforeEach(async () => {
    TestBed.configureTestingModule({
      imports: [FormatButtonComponent],
      providers: [provideTranslocoTesting()],
    });
    fixture = TestBed.createComponent(FormatButtonComponent);
    fixture.componentRef.setInput('formattable', true);
    fixture.componentRef.setInput('language', 'TS');
    fixture.componentRef.setInput('settings', DEFAULT_PRETTIER_SETTINGS);
    formats = 0;
    changes = [];
    fixture.componentInstance.formatRequested.subscribe(() => formats++);
    fixture.componentInstance.settingsChanged.subscribe((change) => changes.push(change));
    document.body.appendChild(fixture.nativeElement);
    fixture.autoDetectChanges();
    await fixture.whenStable();
  });

  function query<T extends HTMLElement>(testid: string): T {
    return fixture.nativeElement.querySelector(`[data-testid="${testid}"]`);
  }

  async function openPanel(): Promise<void> {
    query('editor-format-settings').click();
    await fixture.whenStable();
  }

  it('asks for a format, and names the shortcut', () => {
    query('editor-format').click();

    expect(formats).toBe(1);
    expect(query('editor-format').title).toBe('Formater avec Prettier (Maj+Alt+F)');
  });

  it('opens the library panel on its chevron, into its first field', async () => {
    await openPanel();

    expect(query('editor-format-settings').getAttribute('aria-expanded')).toBe('true');
    expect(query('format-panel').textContent).toContain('pour toute la bibliothèque');
    expect(document.activeElement).toBe(query('prettier-width'));
  });

  it('brings a width back within its range', async () => {
    await openPanel();
    const field = query<HTMLInputElement>('prettier-width');

    field.value = '12';
    field.dispatchEvent(new Event('change'));

    expect(changes).toEqual([{ printWidth: 40 }]);
    expect(field.value).toBe('40');
  });

  it('keeps the width it had when the field holds no number', async () => {
    await openPanel();
    const field = query<HTMLInputElement>('prettier-width');

    field.value = '';
    field.dispatchEvent(new Event('change'));

    expect(changes).toEqual([]);
    expect(field.value).toBe('100');
  });

  it('changes the indentation, the quotes and each switch', async () => {
    await openPanel();

    query('segmented-prettier-indentation').querySelector<HTMLElement>('[data-segment-id="tab"]')!.click();
    query('segmented-prettier-quotes').querySelector<HTMLElement>('[data-segment-id="double"]')!.click();
    query('prettier-formatOnSave').click();
    query('prettier-semicolons').click();

    expect(changes).toEqual([
      { indentation: 'tab' },
      { quotes: 'double' },
      { formatOnSave: true },
      { semicolons: false },
    ]);
  });

  it('shows the current choices', async () => {
    fixture.componentRef.setInput('settings', {
      ...DEFAULT_PRETTIER_SETTINGS,
      indentation: 'four',
      trailingCommas: false,
    });
    await openPanel();

    expect(
      query('segmented-prettier-indentation')
        .querySelector('[aria-checked="true"]')!
        .getAttribute('data-segment-id'),
    ).toBe('four');
    expect(query<HTMLInputElement>('prettier-trailingCommas').checked).toBe(false);
  });

  it('closes on Escape, back on its chevron', async () => {
    await openPanel();

    query('prettier-width').dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
    await fixture.whenStable();

    expect(query('format-panel')).toBeNull();
    expect(document.activeElement).toBe(query('editor-format-settings'));
  });
});
