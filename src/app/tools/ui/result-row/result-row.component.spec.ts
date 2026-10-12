import { ComponentFixture, TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';
import { FakeClipboard } from '@testing/fake-clipboard';
import { provideAppTesting } from '@testing/testing.providers';
import { LONG_RESULT, ResultRowComponent } from './result-row.component';

describe('ResultRowComponent', () => {
  let fixture: ComponentFixture<ResultRowComponent>;
  let clipboard: FakeClipboard;

  beforeEach(async () => {
    clipboard = new FakeClipboard();
    TestBed.configureTestingModule({
      imports: [ResultRowComponent],
      providers: [provideAppTesting({ clipboard })],
    });
    fixture = TestBed.createComponent(ResultRowComponent);
    fixture.componentRef.setInput('name', 'snake_case');
    fixture.componentRef.setInput('value', 'user_id');
    fixture.autoDetectChanges();
    await fixture.whenStable();
  });

  const element = (selector: string): HTMLElement | null => fixture.nativeElement.querySelector(selector);

  it('names the value and copies it from its icon', async () => {
    expect(element('[data-testid="output-row"]')?.dataset['name']).toBe('snake_case');
    expect(element('[data-testid="output-value"]')?.textContent).toBe('user_id');

    element('[data-testid="copy-value"]')!.click();
    await fixture.whenStable();

    expect(clipboard.content).toBe('user_id');
  });

  /**
   * Ten thousand characters of Base64 ran down a strip a third of the card wide, the copy icon
   * floating halfway down it.
   */
  it('gives a long value a block of its own under its name, which the keyboard scrolls', async () => {
    const host: HTMLElement = fixture.nativeElement;
    const value = () => element('[data-testid="output-value"]')!;
    expect(host.classList.contains('long')).toBe(false);
    expect(value().hasAttribute('tabindex')).toBe(false);

    fixture.componentRef.setInput('value', 'A'.repeat(LONG_RESULT));
    await fixture.whenStable();
    expect(host.classList.contains('long')).toBe(false);

    fixture.componentRef.setInput('value', 'A'.repeat(LONG_RESULT + 1));
    await fixture.whenStable();
    expect(host.classList.contains('long')).toBe(true);
    expect(value().getAttribute('tabindex')).toBe('0');
    expect(value().getAttribute('aria-label')).toBe('snake_case');
    // Still copied whole from its icon.
    element('[data-testid="copy-value"]')!.click();
    await fixture.whenStable();
    expect(clipboard.content).toBe('A'.repeat(LONG_RESULT + 1));
  });

  it('copies the exact value behind a rounded one', async () => {
    fixture.componentRef.setInput('copied', 'user_id_exact');
    await fixture.whenStable();

    element('[data-testid="copy-value"]')!.click();
    await fixture.whenStable();

    expect(clipboard.content).toBe('user_id_exact');
  });

  it('says what the value is under its name, and marks the one that matched', async () => {
    fixture.componentRef.setInput('meta', '256 bits');
    fixture.componentRef.setInput('matched', true);
    await fixture.whenStable();

    expect(element('.meta')?.textContent).toBe('256 bits');
    expect(fixture.nativeElement.classList).toContain('matched');
  });

  it('offers no copy for a reading', async () => {
    fixture.componentRef.setInput('copyable', false);
    await fixture.whenStable();

    expect(element('[data-testid="copy-value"]')).toBeNull();
  });
});
