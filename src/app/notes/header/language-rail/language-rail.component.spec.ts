import { ComponentFixture, TestBed } from '@angular/core/testing';
import { By } from '@angular/platform-browser';
import { beforeEach, describe, expect, it } from 'vitest';
import { LanguageTag } from '@core/model/language.model';
import { LanguageBadgeComponent } from '@shared/badges/language-badge/language-badge.component';
import { provideTranslocoTesting } from '@testing/provide-transloco-testing';
import { LanguageRailComponent } from './language-rail.component';

describe('LanguageRailComponent', () => {
  let fixture: ComponentFixture<LanguageRailComponent>;

  function chips(): HTMLButtonElement[] {
    return fixture.debugElement
      .queryAll(By.css('.language-chip'))
      .map((chip) => chip.nativeElement as HTMLButtonElement);
  }

  beforeEach(() => {
    TestBed.configureTestingModule({
      imports: [LanguageRailComponent],
      providers: [provideTranslocoTesting()],
    });
    fixture = TestBed.createComponent(LanguageRailComponent);
    fixture.componentRef.setInput('languages', []);
    fixture.componentRef.setInput('activeLanguages', new Set<LanguageTag>());
    fixture.autoDetectChanges();
  });

  it('renders nothing when the space holds no language', () => {
    expect(fixture.debugElement.query(By.css('.language-rail'))).toBeNull();
  });

  it('renders a badge per language, in the order it received them', async () => {
    fixture.componentRef.setInput('languages', ['json', 'yml']);
    await fixture.whenStable();

    const badges = fixture.debugElement
      .queryAll(By.directive(LanguageBadgeComponent))
      .map((badge) => badge.componentInstance as LanguageBadgeComponent);

    expect(badges.map((badge) => badge.language())).toEqual(['json', 'yml']);
  });

  it('marks the selected languages as pressed', async () => {
    fixture.componentRef.setInput('languages', ['json', 'yml']);
    fixture.componentRef.setInput('activeLanguages', new Set<LanguageTag>(['yml']));
    await fixture.whenStable();

    expect(chips().map((chip) => chip.getAttribute('aria-pressed'))).toEqual(['false', 'true']);
  });

  it('groups the chips under a single accessible name', async () => {
    fixture.componentRef.setInput('languages', ['json']);
    await fixture.whenStable();

    const rail = fixture.nativeElement.querySelector('.language-rail');
    expect(rail.getAttribute('role')).toBe('group');
    expect(rail.getAttribute('aria-label')).toBe('Filtrer par format');
  });

  it('emits the language a click landed on', async () => {
    fixture.componentRef.setInput('languages', ['json', 'yml']);
    await fixture.whenStable();
    let emitted: LanguageTag | undefined;
    fixture.componentInstance.languageToggled.subscribe((language) => (emitted = language));

    chips()[1].click();

    expect(emitted).toBe('yml');
  });
});
