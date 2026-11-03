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

  it.each([
    ['py', 'hue-green'],
    ['tf', 'hue-red'],
    ['vue', 'hue-amber'],
    ['md', 'hue-neutral'],
  ])('colours %s by its family', async (language, hue) => {
    fixture.componentRef.setInput('language', language);
    await fixture.whenStable();

    const span = fixture.debugElement.query(By.css('.lang-tag'));
    expect(span.classes[hue]).toBe(true);
  });
});
