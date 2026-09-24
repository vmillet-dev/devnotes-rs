import { ComponentFixture, TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';
import { LanguageTag } from '@core/model/language.model';
import { provideTranslocoTesting } from '@testing/provide-transloco-testing';
import { FacetsPanelComponent } from './facets-panel.component';

describe('FacetsPanelComponent', () => {
  let fixture: ComponentFixture<FacetsPanelComponent>;

  function clear(): HTMLButtonElement | null {
    return fixture.nativeElement.querySelector('[data-testid="clear-filters"]');
  }

  beforeEach(async () => {
    TestBed.configureTestingModule({
      imports: [FacetsPanelComponent],
      providers: [provideTranslocoTesting()],
    });
    fixture = TestBed.createComponent(FacetsPanelComponent);
    fixture.componentRef.setInput('tags', ['api', 'ops']);
    fixture.componentRef.setInput('languages', ['sql'] as LanguageTag[]);
    fixture.componentRef.setInput('activeTags', new Set<string>());
    fixture.componentRef.setInput('activeLanguages', new Set<LanguageTag>());
    fixture.autoDetectChanges();
    await fixture.whenStable();
  });

  it('draws both rails', () => {
    expect(fixture.nativeElement.querySelector('app-tag-rail')).not.toBeNull();
    expect(fixture.nativeElement.querySelector('app-language-rail')).not.toBeNull();
  });

  /** The way out is offered exactly when there is something to leave. */
  it('offers to clear the filters only while something filters', async () => {
    expect(clear()).toBeNull();

    fixture.componentRef.setInput('canClear', true);
    await fixture.whenStable();
    let cleared = 0;
    fixture.componentInstance.clearRequested.subscribe(() => (cleared += 1));
    clear()?.click();

    expect(cleared).toBe(1);
  });
});
