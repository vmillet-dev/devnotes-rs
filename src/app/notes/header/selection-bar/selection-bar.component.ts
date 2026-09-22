import { ChangeDetectionStrategy, Component, computed, inject, input, output, signal } from '@angular/core';
import { TranslocoService, TranslocoPipe } from '@jsverse/transloco';
import { Folder } from '@core/model/folder.model';
import { Space } from '@core/model/space.model';
import { ChoiceMenuComponent, ChoiceOption } from '@notes/ui/choice-menu/choice-menu.component';

/** An entry of the filing menu, and not a folder id: taking a note out is a choice too. */
const UNFILE = '__unfile__';

@Component({
  selector: 'app-selection-bar',
  imports: [TranslocoPipe, ChoiceMenuComponent],
  templateUrl: './selection-bar.component.html',
  styleUrl: './selection-bar.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class SelectionBarComponent {
  readonly count = input.required<number>();
  readonly spaces = input<readonly Space[]>([]);
  readonly folders = input<readonly Folder[]>([]);

  readonly moveRequested = output<string>();
  /** `null` takes the selection out of its folder; the two directions are one control. */
  readonly fileRequested = output<string | null>();
  readonly tagRequested = output<string>();
  /** The label announces the format: Markdown must not be a surprise. */
  readonly copyRequested = output<void>();
  readonly deleteRequested = output<void>();
  readonly cleared = output<void>();

  private readonly transloco = inject(TranslocoService);

  protected readonly spaceChoices = computed<readonly ChoiceOption[]>(() =>
    this.spaces().map((space) => ({ id: space.id, name: space.name })),
  );

  /**
   * ⚠️ "Take it out" is an entry of the same menu, not a second control: the two directions
   * are one command, and `file_notes` takes `null` for one of them.
   */
  // ⚠️ The template gates on `folders()` and not on this: the way out is always here, so
  // this list is never empty and a space with no folder would still draw the control.
  protected readonly folderChoices = computed<readonly ChoiceOption[]>(() => [
    ...this.folders().map((folder) => ({
      id: folder.id,
      name: folder.name,
      colour: folder.colour,
    })),
    { id: UNFILE, name: this.transloco.translate('selection.unfile') },
  ]);

  protected readonly tagDraft = signal('');

  /** Two steps: the WebView blocks on a native `confirm()`. */
  protected readonly confirmingDelete = signal(false);

  protected onMove(spaceId: string | null): void {
    if (spaceId) {
      this.moveRequested.emit(spaceId);
    }
  }

  protected onFile(value: string | null): void {
    if (value) {
      this.fileRequested.emit(value === UNFILE ? null : value);
    }
  }

  protected submitTag(event: Event): void {
    event.preventDefault();
    const tag = this.tagDraft().trim();
    if (!tag) return;

    this.tagRequested.emit(tag);
    this.tagDraft.set('');
  }

  protected onDeleteClick(): void {
    if (!this.confirmingDelete()) {
      this.confirmingDelete.set(true);
      return;
    }
    this.confirmingDelete.set(false);
    this.deleteRequested.emit();
  }
}
