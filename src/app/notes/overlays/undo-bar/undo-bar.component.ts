import { ChangeDetectionStrategy, Component, computed, input, output } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { Reversible } from '@core/state/notes.store';

/** One key per kind: "3 notes deleted" and "3 notes moved" are not the same sentence. */
const MESSAGES: Record<Reversible['kind'], string> = {
  deletion: 'undo.deleted',
  move: 'undo.moved',
  tag: 'undo.tagged',
  file: 'undo.filed',
  arrange: 'undo.arranged',
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

  protected readonly message = computed(() => MESSAGES[this.action().kind]);
}
