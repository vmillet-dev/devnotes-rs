import { ChangeDetectionStrategy, Component, computed, inject } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { ClockService } from '@core/services/time/clock.service';
import { NoteRevisionsStore } from '@core/state/note-revisions.store';
import { relativeTimeRef } from '@core/utils/relative-time.util';

/**
 * The bodies kept beside this note: a row opens a preview, and the preview goes back.
 *
 * ⚠️ The trash protects a deletion; nothing protected an edit. The point is not the
 * restoring — it is the **ease**: a text you know is recoverable is a text you edit
 * freely, and touching a snippet that works stops costing nerve.
 */
@Component({
  selector: 'app-revision-panel',
  imports: [TranslocoPipe],
  templateUrl: './revision-panel.component.html',
  styleUrl: './revision-panel.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class RevisionPanelComponent {
  protected readonly store = inject(NoteRevisionsStore);

  private readonly clock = inject(ClockService);

  /** ⚠️ Formatted here rather than in Rust: a label has to age without a round trip. */
  protected readonly rows = computed(() =>
    this.store.revisions().map((revision) => ({
      revision,
      when: relativeTimeRef(revision.takenAt, this.clock.now()),
    })),
  );

  protected readonly preview = computed(() => {
    const shown = this.store.preview();
    return shown && { ...shown, when: relativeTimeRef(shown.revision.takenAt, this.clock.now()) };
  });
}
