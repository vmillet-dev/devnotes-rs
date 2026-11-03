import { ChangeDetectionStrategy, Component, computed, input, output, signal } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { DialogComponent } from '@shared/layout/dialog/dialog.component';
import { TagUsage } from '@core/model/note.model';
import { PendingTagChange } from '@core/state/tags.store';

/** One key per kind: a merge is the only one that cannot be undone, and says so. */
const CONFIRMATIONS: Record<PendingTagChange['kind'], string> = {
  rename: 'tagManager.confirmRename',
  merge: 'tagManager.confirmMerge',
  delete: 'tagManager.confirmDelete',
};

/** One field for renaming and merging: in the database it is the same operation. */
@Component({
  selector: 'app-tag-manager',
  imports: [DialogComponent, TranslocoPipe],
  templateUrl: './tag-manager.component.html',
  styleUrl: './tag-manager.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class TagManagerComponent {
  readonly tags = input.required<readonly TagUsage[]>();
  readonly selected = input.required<ReadonlySet<string>>();
  readonly isLoading = input(false);
  readonly pending = input<PendingTagChange | null>(null);

  readonly closed = output<void>();
  readonly toggled = output<string>();
  readonly renameRequested = output<string>();
  readonly deleteRequested = output<void>();
  readonly confirmed = output<void>();
  readonly cancelled = output<void>();

  protected readonly target = signal('');

  protected readonly confirmation = computed(() => {
    const change = this.pending();
    if (change === null) return null;

    return {
      key: CONFIRMATIONS[change.kind],
      params: {
        count: change.notes,
        tags: change.tags.length,
        tag: change.tags[0] ?? '',
        into: change.into,
      },
    };
  });

  protected readonly selectionCount = computed(() => this.selected().size);
  protected readonly canApply = computed(() => this.selectionCount() > 0);

  /** One tag ticked is a rename. Several is a merge. */
  protected readonly applyKey = computed(() =>
    this.selectionCount() > 1 ? 'tagManager.merge' : 'tagManager.rename',
  );

  protected submit(event: Event): void {
    event.preventDefault();
    const target = this.target().trim();
    if (!target || !this.canApply()) return;

    this.renameRequested.emit(target);
  }

  protected onDeleteClick(): void {
    if (!this.canApply()) return;

    this.deleteRequested.emit();
  }

  protected onConfirm(): void {
    this.confirmed.emit();
    this.target.set('');
  }
}
