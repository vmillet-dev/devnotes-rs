import { TestBed } from '@angular/core/testing';
import { describe, expect, it } from 'vitest';
import { SpinnerComponent } from './spinner.component';

describe('SpinnerComponent', () => {
  /** The sentence beside it speaks; a turning ring announced would only be noise. */
  it('is hidden from a screen reader and holds nothing to read', () => {
    const fixture = TestBed.createComponent(SpinnerComponent);
    fixture.detectChanges();
    const host = fixture.nativeElement as HTMLElement;

    expect(host.getAttribute('aria-hidden')).toBe('true');
    expect(host.textContent).toBe('');
  });
});
