import { ComponentFixture, TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { FakeClipboard } from '@testing/fake-clipboard';
import { provideAppTesting } from '@testing/testing.providers';
import { OutputRowComponent } from './output-row.component';

describe('OutputRowComponent', () => {
  let fixture: ComponentFixture<OutputRowComponent>;
  let clipboard: FakeClipboard;

  beforeEach(async () => {
    TestBed.resetTestingModule();
    clipboard = new FakeClipboard();
    TestBed.configureTestingModule({
      imports: [OutputRowComponent],
      providers: [provideAppTesting({ clipboard })],
    });
    fixture = TestBed.createComponent(OutputRowComponent);
    fixture.componentRef.setInput('name', 'HMAC-SHA256');
    fixture.componentRef.setInput('value', 'bfc0d2dc');
    fixture.componentRef.setInput('meta', '256 bits');
    fixture.autoDetectChanges();
    await fixture.whenStable();
  });

  const copy = (): HTMLButtonElement => fixture.nativeElement.querySelector('[data-testid="copy-value"]');

  it('shows the value under its name and what it measures', () => {
    expect(fixture.nativeElement.textContent).toContain('HMAC-SHA256');
    expect(fixture.nativeElement.textContent).toContain('256 bits');
    expect(fixture.nativeElement.querySelector('[data-testid="output-value"]').textContent).toBe('bfc0d2dc');
  });

  it('copies the value, and names what it copies for a screen reader', async () => {
    expect(copy().getAttribute('aria-label')).toBe('Copier HMAC-SHA256');

    copy().click();

    // The acknowledgement comes after the write resolves, and whenStable does not wait for it.
    await vi.waitFor(() => expect(copy().classList).toContain('copied'));
    expect(clipboard.content).toBe('bfc0d2dc');
  });

  it('copies nothing when there is nothing', async () => {
    fixture.componentRef.setInput('value', '');
    await fixture.whenStable();

    expect(copy().disabled).toBe(true);
  });

  it('is outlined when it answers what was asked', async () => {
    fixture.componentRef.setInput('matched', true);
    await fixture.whenStable();

    expect(fixture.nativeElement.classList).toContain('matched');
  });
});
