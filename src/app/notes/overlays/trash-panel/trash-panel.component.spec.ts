import { ComponentFixture, TestBed } from '@angular/core/testing';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { TrashedNote } from '@core/model/note.model';
import { provideTranslocoTesting } from '@testing/provide-transloco-testing';
import { TrashPanelComponent } from './trash-panel.component';

const NOW = new Date('2026-08-27T09:00:00Z');

function trashed(overrides: Partial<TrashedNote> = {}): TrashedNote {
  return {
    id: 'note-1',
    spaceId: 'space-1',
    title: 'Deleted note',
    language: 'txt',
    content: 'line one\nline two\nline three',
    tags: [],
    deletedAt: new Date('2026-08-27T08:00:00Z'),
    purgeAt: new Date('2026-09-26T08:00:00Z'),
    kind: 'snippet',
    ...overrides,
  };
}

describe('TrashPanelComponent', () => {
  let fixture: ComponentFixture<TrashPanelComponent>;

  function rows(): HTMLElement[] {
    return [...fixture.nativeElement.querySelectorAll('.trash-row')];
  }

  function actionsOf(index: number): HTMLButtonElement[] {
    return [...rows()[index].querySelectorAll<HTMLButtonElement>('.trash-action')];
  }

  beforeEach(async () => {
    // Only `Date`: a faked `requestAnimationFrame` blocks the zoneless scheduler.
    vi.useFakeTimers({ toFake: ['Date'] });
    vi.setSystemTime(NOW);

    TestBed.resetTestingModule();
    TestBed.configureTestingModule({
      imports: [TrashPanelComponent],
      providers: [provideTranslocoTesting()],
    });
    fixture = TestBed.createComponent(TrashPanelComponent);
    fixture.componentRef.setInput('notes', [trashed()]);
    fixture.autoDetectChanges();
    await fixture.whenStable();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('states the retention rule the backend applies', () => {
    expect(fixture.nativeElement.querySelector('.trash-hint').textContent).toContain('30 jours');
  });

  it('shows when a note was deleted and when it will be erased', () => {
    const meta = rows()[0].querySelector('.trash-row-meta')?.textContent ?? '';

    expect(meta).toContain('il y a 1h');
    expect(meta).toContain('30');
  });

  /**
   * The panel says notes are kept 30 days and this said 31 one line under it. The cause
   * is not arithmetic on the retention — it is that `ClockService` ticks every 30 s, so the
   * `now` a card renders against can be **behind** an instant Rust has just stamped. The
   * gap then reads as 30 days *and change*, and rounding up made the change a whole day.
   */
  it('agrees with the retention rule stated above it, on a clock that has not ticked yet', async () => {
    // Deleted 20 seconds after the clock last looked: `purgeAt` is that plus 30 days.
    fixture.componentRef.setInput('notes', [
      trashed({
        deletedAt: new Date('2026-08-27T09:00:20Z'),
        purgeAt: new Date('2026-09-26T09:00:20Z'),
      }),
    ]);
    await fixture.whenStable();

    const meta = rows()[0].querySelector('.trash-row-meta')?.textContent ?? '';
    expect(meta).toContain('30');
    expect(meta).not.toContain('31');
  });

  it('counts the last day as still the user’s', async () => {
    fixture.componentRef.setInput('notes', [trashed({ purgeAt: new Date('2026-08-27T23:00:00Z') })]);
    await fixture.whenStable();

    expect(rows()[0].querySelector('.trash-row-meta')?.textContent).toContain('1 j');
  });

  it('says a note is erased today once its deadline has passed', async () => {
    fixture.componentRef.setInput('notes', [trashed({ purgeAt: new Date('2026-08-27T08:59:00Z') })]);
    await fixture.whenStable();

    expect(rows()[0].querySelector('.trash-row-meta')?.textContent).toContain("aujourd'hui");
  });

  it('falls back to a label for an untitled note', async () => {
    fixture.componentRef.setInput('notes', [trashed({ title: '' })]);
    await fixture.whenStable();

    expect(rows()[0].querySelector('.trash-row-title')?.textContent?.trim()).toBe('Sans titre');
  });

  it('shows an empty state rather than an empty list', async () => {
    fixture.componentRef.setInput('notes', []);
    await fixture.whenStable();

    expect(fixture.nativeElement.querySelector('.trash-state').textContent).toContain('vide');
    expect(fixture.nativeElement.querySelector('.trash-footer')).toBeNull();
  });

  it('restores in one click', async () => {
    let emitted: string | undefined;
    fixture.componentInstance.restoreRequested.subscribe((id) => (emitted = id));

    actionsOf(0)[0].click();
    await fixture.whenStable();

    expect(emitted).toBe('note-1');
  });

  it('asks for a confirmation before erasing for good', async () => {
    let emitted = 0;
    fixture.componentInstance.purgeRequested.subscribe(() => (emitted += 1));

    actionsOf(0)[1].click();
    await fixture.whenStable();
    expect(emitted).toBe(0);
    expect(actionsOf(0)[1].textContent).toContain('Confirmer');

    actionsOf(0)[1].click();
    await fixture.whenStable();
    expect(emitted).toBe(1);
  });

  it('confirms one row at a time', async () => {
    fixture.componentRef.setInput('notes', [trashed(), trashed({ id: 'note-2' })]);
    await fixture.whenStable();

    actionsOf(0)[1].click();
    await fixture.whenStable();

    expect(actionsOf(1)[1].textContent).not.toContain('Confirmer');
  });

  describe('emptying the whole trash', () => {
    function emptyButton(): HTMLButtonElement {
      return fixture.nativeElement.querySelector('.trash-footer .trash-action');
    }

    const find = (testId: string): HTMLButtonElement | null =>
      fixture.nativeElement.querySelector(`[data-testid="${testId}"]`);

    /**
     * Nothing puts these notes back, so the confirmation is not the same button in the
     * same place — that is the shape a double click defeats. It says how many it would
     * erase first, like the tag manager does before a merge.
     */
    it('says what it would erase, and confirms on another button', async () => {
      fixture.componentRef.setInput('notes', [trashed(), trashed({ id: 'n-2' })]);
      await fixture.whenStable();
      let emitted = 0;
      fixture.componentInstance.emptyRequested.subscribe(() => (emitted += 1));

      emptyButton().click();
      await fixture.whenStable();

      expect(emitted).toBe(0);
      expect(find('trash-empty')).toBeNull();
      expect(find('trash-empty-warning')?.textContent).toContain('2');

      find('trash-empty-confirm')?.click();
      await fixture.whenStable();

      expect(emitted).toBe(1);
      expect(find('trash-empty')).not.toBeNull();
    });

    it('withdraws the offer rather than leaving it armed', async () => {
      let emitted = 0;
      fixture.componentInstance.emptyRequested.subscribe(() => (emitted += 1));

      emptyButton().click();
      await fixture.whenStable();
      find('trash-empty-cancel')?.click();
      await fixture.whenStable();

      expect(emitted).toBe(0);
      expect(find('trash-empty-warning')).toBeNull();
    });

    it('is not offered when there is nothing to erase', async () => {
      fixture.componentRef.setInput('notes', []);
      await fixture.whenStable();

      expect(fixture.nativeElement.querySelector('.trash-footer')).toBeNull();
    });
  });
});
