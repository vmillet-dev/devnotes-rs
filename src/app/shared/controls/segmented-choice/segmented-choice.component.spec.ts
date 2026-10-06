import { ComponentFixture, TestBed } from '@angular/core/testing';
import { TranslocoService } from '@jsverse/transloco';
import { beforeEach, describe, expect, it } from 'vitest';
import { provideTranslocoTesting } from '@testing/provide-transloco-testing';
import { Segment, SegmentedChoiceComponent } from './segmented-choice.component';

const DENSITIES: readonly Segment[] = [
  { id: 'comfortable', labelKey: 'settings.density.comfortable' },
  { id: 'compact', labelKey: 'settings.density.compact' },
];

describe('SegmentedChoiceComponent', () => {
  let fixture: ComponentFixture<SegmentedChoiceComponent>;

  function segments(): HTMLButtonElement[] {
    return [...fixture.nativeElement.querySelectorAll('[data-testid="segment"]')];
  }

  function labels(): string[] {
    return segments().map((segment) => segment.textContent?.trim() ?? '');
  }

  beforeEach(async () => {
    TestBed.resetTestingModule();
    TestBed.configureTestingModule({
      imports: [SegmentedChoiceComponent],
      providers: [provideTranslocoTesting()],
    });
    fixture = TestBed.createComponent(SegmentedChoiceComponent);
    fixture.componentRef.setInput('kind', 'density');
    fixture.componentRef.setInput('label', 'Densité');
    fixture.componentRef.setInput('segments', DENSITIES);
    fixture.componentRef.setInput('currentId', 'comfortable');
    fixture.autoDetectChanges();
    await fixture.whenStable();
  });

  it('shows a disabled segment, and chooses nothing from it', async () => {
    const chosen: string[] = [];
    fixture.componentInstance.chosen.subscribe((id) => chosen.push(id));
    fixture.componentRef.setInput('disabledIds', ['compact']);
    await fixture.whenStable();

    segments()[1]!.click();

    expect(segments()[1]!.getAttribute('aria-disabled')).toBe('true');
    expect(segments()[0]!.getAttribute('aria-disabled')).toBeNull();
    expect(chosen).toEqual([]);
  });

  it('checks exactly the current segment', () => {
    expect(segments().map((segment) => segment.getAttribute('aria-checked'))).toEqual(['true', 'false']);
  });

  it('emits another segment, and nothing for the one already chosen', () => {
    const chosen: string[] = [];
    fixture.componentInstance.chosen.subscribe((id: string) => chosen.push(id));

    segments()[0].click();
    segments()[1].click();

    expect(chosen).toEqual(['compact']);
  });

  it('translates its labels, and follows a change of language', async () => {
    expect(labels()).toEqual(['Confortable', 'Compacte']);

    TestBed.inject(TranslocoService).setActiveLang('en');
    await fixture.whenStable();

    expect(labels()).toEqual(['Comfortable', 'Compact']);
  });
});
