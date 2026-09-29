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
    fixture.componentRef.setInput('kindCounts', [{ value: 'note', count: 1 }]);
    fixture.componentRef.setInput('activeKinds', new Set());
    fixture.componentRef.setInput('priorityCounts', [{ value: 'high', count: 1 }]);
    fixture.componentRef.setInput('activePriorities', new Set());
    fixture.autoDetectChanges();
    await fixture.whenStable();
  });

  it('draws the four rails, the kinds and the priorities first', () => {
    const rails = [...fixture.nativeElement.querySelector('.facets-panel').children].map((rail: Element) =>
      rail.tagName.toLowerCase(),
    );

    expect(rails).toEqual(['app-kind-rail', 'app-priority-rail', 'app-tag-rail', 'app-language-rail']);
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
