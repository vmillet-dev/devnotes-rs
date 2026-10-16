import { ComponentFixture, TestBed } from '@angular/core/testing';
import { By } from '@angular/platform-browser';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { provideTranslocoTesting } from '@testing/provide-transloco-testing';
import { LifecycleBadgeComponent } from './lifecycle-badge.component';

describe('LifecycleBadgeComponent', () => {
  let fixture: ComponentFixture<LifecycleBadgeComponent>;

  function text(): string {
    return fixture.nativeElement.textContent.replace(/\s+/g, ' ').trim();
  }

  beforeEach(() => {
    // Only virtualize `Date`; Angular's zoneless scheduler relies on real rAF/setTimeout for `whenStable()` to resolve.
    vi.useFakeTimers({ toFake: ['Date'] });
    vi.setSystemTime(new Date('2026-01-01T00:00:00Z'));

    TestBed.configureTestingModule({
      imports: [LifecycleBadgeComponent],
      providers: [provideTranslocoTesting()],
    });
    fixture = TestBed.createComponent(LifecycleBadgeComponent);
    fixture.componentRef.setInput('lifecycle', { kind: 'permanent' });
    fixture.autoDetectChanges();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('shows the "never expires" label for a permanent note', () => {
    expect(text()).toBe('∞ N’expire jamais');
  });

  it('hides the decorative pictogram from assistive tech', () => {
    expect(fixture.nativeElement.querySelector('[aria-hidden="true"]').textContent).toBe('∞');
  });

  it('is never stale for a permanent note', () => {
    expect(fixture.debugElement.query(By.css('.lifecycle-badge')).classes['stale']).toBeFalsy();
  });

  it('shows the countdown label for an expiring note', async () => {
    fixture.componentRef.setInput('lifecycle', { kind: 'expires', at: new Date('2026-01-04T00:00:00Z') });
    await fixture.whenStable();

    expect(text()).toBe('⏳ expire dans 3j');
  });

  it('is marked stale when the backend reports the expiry as near', async () => {
    fixture.componentRef.setInput('lifecycle', { kind: 'expires', at: new Date('2026-01-02T00:00:00Z') });
    fixture.componentRef.setInput('expiringSoon', true);
    await fixture.whenStable();

    expect(fixture.debugElement.query(By.css('.lifecycle-badge')).classes['stale']).toBe(true);
  });

  it('is not marked stale when the backend did not flag the expiry', async () => {
    fixture.componentRef.setInput('lifecycle', { kind: 'expires', at: new Date('2026-02-01T00:00:00Z') });
    await fixture.whenStable();

    expect(fixture.debugElement.query(By.css('.lifecycle-badge')).classes['stale']).toBeFalsy();
  });
});
