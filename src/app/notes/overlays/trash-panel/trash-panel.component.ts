import { ChangeDetectionStrategy, Component, computed, inject, input, output, signal } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { TranslationRef } from '@core/services/i18n/translation-ref.model';
import { ClockService } from '@core/services/time/clock.service';
import { relativeTimeRef } from '@core/utils/relative-time.util';
import { DialogComponent } from '@shared/layout/dialog/dialog.component';
import { TrashedNote } from '@core/model/note.model';

const MS_PER_DAY = 24 * 60 * 60 * 1000;
const SNIPPET_LINES = 2;

interface TrashRow {
  readonly note: TrashedNote;
  /** Empty for a todo list: its items do not travel this far. */
  readonly snippet: string;
  readonly snippetKey: string | null;
  readonly deletedRef: TranslationRef;
  readonly purgeRef: TranslationRef;
}

/** Computed at render, like the cards' relative times: it has to age without a round trip. */
@Component({
  selector: 'app-trash-panel',
  imports: [DialogComponent, TranslocoPipe],
  templateUrl: './trash-panel.component.html',
  styleUrl: './trash-panel.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class TrashPanelComponent {
  private readonly clock = inject(ClockService);

  readonly notes = input.required<readonly TrashedNote[]>();
  readonly isLoading = input(false);

  readonly closed = output<void>();
  readonly restoreRequested = output<string>();
  readonly purgeRequested = output<string>();
  readonly emptyRequested = output<void>();

  /** One confirmation per row: the id awaiting it, or `null`. */
  protected readonly confirmingPurge = signal<string | null>(null);
  protected readonly confirmingEmpty = signal(false);

  protected readonly rows = computed<readonly TrashRow[]>(() => {
    const now = this.clock.now();

    return this.notes().map((note) => ({
      note,
      snippet: note.content.split('\n').slice(0, SNIPPET_LINES).join('\n'),
      snippetKey: note.kind === 'checklist' ? 'trash.checklistNote' : null,
      deletedRef: relativeTimeRef(note.deletedAt, now),
      purgeRef: purgeRef(note.purgeAt, now),
    }));
  });

  /**
   * ⚠️ Two strings and not one with a count: French keeps the singular at one where English
   * does not, and this sentence is the last thing said before the notes stop existing.
   */
  protected readonly emptyWarning = computed<TranslationRef>(() => {
    const count = this.notes().length;

    return count === 1
      ? { key: 'trash.emptyTrashWarningOne' }
      : { key: 'trash.emptyTrashWarning', params: { count } };
  });

  protected onPurgeClick(id: string): void {
    if (this.confirmingPurge() !== id) {
      this.confirmingPurge.set(id);
      return;
    }
    this.confirmingPurge.set(null);
    this.purgeRequested.emit(id);
  }

  protected confirmEmpty(): void {
    this.confirmingEmpty.set(false);
    this.emptyRequested.emit();
  }
}

/**
 * ⚠️ Rounded to the nearest day, and neither of the other two will do. Rounding **up** put
 * "erased in 31 d" one line under a panel saying notes are kept 30 days: `purgeAt` is
 * `deletedAt + 30 days`, so a note just deleted has a few milliseconds under 30 left and
 * any fraction became a whole extra day. Rounding **down** says 29 for the same note, which
 * is the same contradiction the other way round.
 */
function purgeRef(purgeAt: Date, now: Date): TranslationRef {
  const days = Math.round((purgeAt.getTime() - now.getTime()) / MS_PER_DAY);

  return days <= 0 ? { key: 'trash.purgesToday' } : { key: 'trash.purgesIn', params: { count: days } };
}
