import { ComponentFixture, TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';
import { NotesViewMode } from '@core/model/board.model';
import { provideTranslocoTesting } from '@testing/provide-transloco-testing';
import { ViewSwitchComponent } from './view-switch.component';

describe('ViewSwitchComponent', () => {
  let fixture: ComponentFixture<ViewSwitchComponent>;

  function option(which: 'date' | 'board'): HTMLButtonElement {
    return fixture.nativeElement.querySelector(`[data-testid="view-${which}"]`);
  }

  beforeEach(() => {
    TestBed.resetTestingModule();
    TestBed.configureTestingModule({
      imports: [ViewSwitchComponent],
      providers: [provideTranslocoTesting()],
    });
    fixture = TestBed.createComponent(ViewSwitchComponent);
    fixture.componentRef.setInput('mode', 'date');
    fixture.autoDetectChanges();
  });

  it('marks the showing view, and only it', () => {
    expect(option('date').getAttribute('aria-pressed')).toBe('true');
    expect(option('board').getAttribute('aria-pressed')).toBe('false');
  });

  it('asks for the board without switching itself', async () => {
    const seen: NotesViewMode[] = [];
    fixture.componentInstance.modeChanged.subscribe((mode) => seen.push(mode));

    option('board').click();
    await fixture.whenStable();

    expect(seen).toEqual(['board']);
    // The store decides; the switch only asks.
    expect(option('date').getAttribute('aria-pressed')).toBe('true');
  });

  /** Disabled with a reason, not hidden: a control that vanishes reads as a bug. */
  it('disables the board and says why when no space is chosen', async () => {
    fixture.componentRef.setInput('boardAvailable', false);
    await fixture.whenStable();

    expect(option('board').disabled).toBe(true);
    expect(option('board').getAttribute('title')).toContain('espace');
    expect(option('date').disabled).toBe(false);
  });
});
