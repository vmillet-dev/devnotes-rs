import { Injectable, computed, inject, signal } from '@angular/core';
import { ClipboardService } from '@core/services/clipboard/clipboard.service';
import { DesktopNotifier } from '@core/services/notifications/desktop-notifier.service';
import { ErrorNotifier } from '@core/services/errors/error-notifier.service';
import { SettingsStore } from '@core/services/settings/settings.store';
import { SEARCH_DEBOUNCE_MS, debounced } from '@core/services/time/debounce';
import { ClockService } from '@core/services/time/clock.service';
import { AppWindowService } from '@core/services/window/app-window.service';
import { NotesRepository } from '../data/notes.repository';
import { noteCopyText } from '../model/checklist.model';
import { Note } from '../model/note.model';

/** Beyond this the list no longer fits on screen and the keyboard loses it. */
const MAX_RESULTS = 8;

/**
 * Queries every space and ignores the canvas filters. It does not reuse `NotesStore`
 * for the same reason: its search would change what the canvas shows behind it.
 */
@Injectable({ providedIn: 'root' })
export class PaletteStore {
  private readonly repository = inject(NotesRepository);
  private readonly clipboard = inject(ClipboardService);
  private readonly clock = inject(ClockService);
  private readonly window = inject(AppWindowService);
  private readonly notifier = inject(ErrorNotifier);
  private readonly desktop = inject(DesktopNotifier);
  private readonly settings = inject(SettingsStore);

  private readonly _isOpen = signal(false);
  private readonly _query = signal('');
  private readonly _results = signal<readonly Note[]>([]);
  private readonly _highlighted = signal(0);
  private readonly _pendingFill = signal<Note | null>(null);

  readonly isOpen = this._isOpen.asReadonly();
  readonly query = this._query.asReadonly();
  readonly results = this._results.asReadonly();
  readonly pendingFill = this._pendingFill.asReadonly();

  /** The create row comes after the results: finding a snippet keeps first place. */
  readonly canCreate = computed(() => this._query().trim().length > 0);

  readonly optionCount = computed(() => this._results().length + (this.canCreate() ? 1 : 0));

  /** Bounded: a shrinking list must not leave the index outside it. */
  readonly highlighted = computed(() => Math.min(this._highlighted(), Math.max(0, this.optionCount() - 1)));

  readonly isCreateHighlighted = computed(
    () => this.canCreate() && this.highlighted() === this._results().length,
  );

  readonly highlightedNote = computed<Note | null>(() => this._results()[this.highlighted()] ?? null);

  private readonly scheduleSearch = debounced((query: string) => void this.search(query), SEARCH_DEBOUNCE_MS);

  /** Opens on the most recent notes: reopening without typing has a meaning. */
  async open(): Promise<void> {
    this._isOpen.set(true);
    this._query.set('');
    this._pendingFill.set(null);
    this._highlighted.set(0);
    await this.search('');
  }

  close(): void {
    this.scheduleSearch.cancel();
    this._isOpen.set(false);
    this._pendingFill.set(null);
  }

  setQuery(query: string): void {
    this._query.set(query);
    this._highlighted.set(0);
    this.scheduleSearch(query);
  }

  /** Stops at both ends: wrapping around would lose track of where one is. */
  moveHighlight(step: number): void {
    const last = this.optionCount() - 1;
    this._highlighted.set(Math.max(0, Math.min(last, this.highlighted() + step)));
  }

  highlight(index: number): void {
    this._highlighted.set(index);
  }

  /** It does not know `NotesStore`, and the other way round would be a cycle. */
  takeNewNoteContent(): string | null {
    if (!this.isCreateHighlighted()) return null;

    const content = this._query().trim();
    this.close();

    return content;
  }

  /** A snippet with fields goes through the form first, or the copy is unusable. */
  async chooseHighlighted(): Promise<void> {
    const note = this.highlightedNote();
    if (!note) return;

    if (note.placeholders.length > 0) {
      this._pendingFill.set(note);
      return;
    }

    // A todo list has no content: without this the palette would copy an empty string.
    await this.copyAndDismiss(noteCopyText(note), note.title);
  }

  /**
   * ⚠️ The toast is sent **after** the window has gone, and it is the only
   * acknowledgement this path can have: `StatusNotifier` draws under the titlebar, and
   * the titlebar is what is being taken away. Without it the window simply vanished, and
   * it read as the application crashing on a copy that had in fact worked (#285).
   */
  async copyAndDismiss(content: string, title: string): Promise<void> {
    if (!(await this.clipboard.copy(content))) {
      this.notifier.notify({ ref: { key: 'errors.copyFailed' } });
      return;
    }

    this.close();
    // The window disappears: the user goes back to paste where they were.
    await this.window.hide();

    await this.desktop.notify(
      { key: 'palette.copiedTitle' },
      title ? { key: 'palette.copiedNote', params: { title } } : { key: 'palette.copiedUntitled' },
    );
  }

  cancelFill(): void {
    this._pendingFill.set(null);
  }

  private async search(query: string): Promise<void> {
    const now = this.clock.now();

    try {
      const view = await this.repository.query({
        spaceId: null,
        folderId: null,
        search: query.trim(),
        filter: 'all',
        tags: [],
        languages: [],
        now,
        tzOffsetMinutes: now.getTimezoneOffset(),
        // The one place the pinned hoist is a setting.
        pinnedFirst: this.settings.showPinnedFirst(),
      });

      this._results.set(view.sections.flatMap((section) => [...section.notes]).slice(0, MAX_RESULTS));
    } catch (error) {
      this.notifier.reportFailure('errors.notesLoadFailed', error);
      this._results.set([]);
    }
  }
}
