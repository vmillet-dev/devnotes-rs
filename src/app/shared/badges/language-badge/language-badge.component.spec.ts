import { ComponentFixture, TestBed } from '@angular/core/testing';
import { By } from '@angular/platform-browser';
import { beforeEach, describe, expect, it } from 'vitest';
import { LanguageBadgeComponent } from './language-badge.component';

describe('LanguageBadgeComponent', () => {
  let fixture: ComponentFixture<LanguageBadgeComponent>;

  beforeEach(() => {
    TestBed.configureTestingModule({ imports: [LanguageBadgeComponent] });
    fixture = TestBed.createComponent(LanguageBadgeComponent);
    fixture.componentRef.setInput('language', 'json');
    fixture.autoDetectChanges();
  });

  it('renders the human-readable label for the given language', () => {
    expect(fixture.nativeElement.textContent.trim()).toBe('JSON');
  });

  it('applies a class matching the language tag', async () => {
    fixture.componentRef.setInput('language', 'py');
    await fixture.whenStable();

    const span = fixture.debugElement.query(By.css('.lang-tag'));
    expect(span.classes['lang-py']).toBe(true);
  });
});
