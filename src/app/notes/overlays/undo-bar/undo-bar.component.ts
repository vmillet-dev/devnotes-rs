import { ChangeDetectionStrategy, Component, computed, input, output } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { NoteKind } from '@core/model/note.model';
import { Reversible } from '@core/state/undo.store';

/** One key per kind: "3 notes deleted" and "3 notes moved" are not the same sentence. */
const MESSAGES: Record<Exclude<Reversible['kind'], 'convert' | 'folderMove'>, string> = {
  deletion: 'undo.deleted',
  move: 'undo.moved',
  tag: 'undo.tagged',
  untag: 'undo.untagged',
  pin: 'undo.pinned',
  priority: 'undo.prioritised',
  file: 'undo.filed',
  arrange: 'undo.arranged',
};

const CONVERTED: Record<NoteKind, string> = {
  note: 'undo.convertedInto.note',
  snippet: 'undo.convertedInto.snippet',
  checklist: 'undo.convertedInto.checklist',
  calc: 'undo.convertedInto.calc',
  drawing: 'undo.convertedInto.drawing',
};

/** `role="status"` and not `alert`: a screen reader must not interrupt for this. */
@Component({
  selector: 'app-undo-bar',
  imports: [TranslocoPipe],
  templateUrl: './undo-bar.component.html',
  styleUrl: './undo-bar.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class UndoBarComponent {
  readonly action = input.required<Reversible>();

  readonly undone = output<void>();
  readonly dismissed = output<void>();

  protected readonly message = computed(() => {
    const action = this.action();
    if (action.kind === 'convert') return CONVERTED[action.into];
    if (action.kind === 'folderMove')
      return action.into === null ? 'undo.folderMovedTop' : 'undo.folderMoved';
    return action.kind === 'pin' && !action.pinned ? 'undo.unpinned' : MESSAGES[action.kind];
  });

  /** A count for the batches; a folder moved says which, and where. */
  protected readonly params = computed<Record<string, string | number>>(() => {
    const action = this.action();
    return action.kind === 'folderMove'
      ? { name: action.name, into: action.into ?? '' }
      : { count: action.count };
  });
}
