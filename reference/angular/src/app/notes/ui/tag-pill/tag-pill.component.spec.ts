import { ComponentFixture, TestBed } from '@angular/core/testing';
import { By } from '@angular/platform-browser';
import { beforeEach, describe, expect, it } from 'vitest';
import { TagPillComponent } from './tag-pill.component';

describe('TagPillComponent', () => {
  let fixture: ComponentFixture<TagPillComponent>;

  beforeEach(() => {
    TestBed.configureTestingModule({ imports: [TagPillComponent] });
    fixture = TestBed.createComponent(TagPillComponent);
    fixture.componentRef.setInput('label', 'urgent');
    fixture.autoDetectChanges();
  });

  it('renders the label prefixed with a hash', () => {
    expect(fixture.nativeElement.textContent.trim()).toBe('#urgent');
  });

  it('is not marked active by default', () => {
    const button = fixture.debugElement.query(By.css('button'));
    expect(button.classes['on']).toBeFalsy();
    expect(button.nativeElement.getAttribute('aria-pressed')).toBe('false');
  });

  it('exposes the active state as a pressed toggle', async () => {
    fixture.componentRef.setInput('active', true);
    await fixture.whenStable();

    const button = fixture.debugElement.query(By.css('button'));
    expect(button.classes['on']).toBe(true);
    expect(button.nativeElement.getAttribute('aria-pressed')).toBe('true');
  });

  it('renders a plain label rather than a button when not interactive', async () => {
    fixture.componentRef.setInput('interactive', false);
    await fixture.whenStable();

    expect(fixture.debugElement.query(By.css('button'))).toBeNull();
    expect(fixture.nativeElement.querySelector('span.tag-pill.static').textContent.trim()).toBe('#urgent');
  });

  it('emits the label when clicked while interactive', () => {
    let emitted: string | undefined;
    fixture.componentInstance.toggled.subscribe((label) => (emitted = label));

    fixture.debugElement.query(By.css('button')).triggerEventHandler('click');

    expect(emitted).toBe('urgent');
  });
});
