import { ComponentFixture, TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';
import { FakeClipboard } from '@testing/fake-clipboard';
import { provideAppTesting } from '@testing/testing.providers';
import { ResultRowComponent } from './result-row.component';

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
