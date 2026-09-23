import {
  ChangeDetectionStrategy,
  Component,
  ElementRef,
  inject,
  input,
  output,
  signal,
  viewChild,
} from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { Folder, FolderColour } from '@core/model/folder.model';
import { FoldersStore } from '@core/state/folders.store';

/** The palette the back end assigns from; the order is `FolderColour::ALL`. */
const COLOURS: readonly FolderColour[] = ['blue', 'amber', 'purple', 'green', 'red'];

/**
 * Rename, recolour, delete — the three things a folder can be told to do, in one panel so
 * the switcher, the breadcrumb, the rail and the zone menu cannot drift apart. It performs
 * them itself; its host only hears `finished`, to close whatever it was opened from.
 */
@Component({
  selector: 'app-folder-editor',
  imports: [TranslocoPipe],
  templateUrl: './folder-editor.component.html',
  styleUrl: './folder-editor.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class FolderEditorComponent {
  readonly folder = input.required<Folder>();

  /**
   * How many of this folder's notes are on screen, or `null` where selecting them makes
   * no sense.
   *
   * ⚠️ Opt-in rather than always drawn: the switcher lists folders you are **not** in, and
   * "select all the notes of that one over there" is a gesture with no visible result. The
   * zone menu and the breadcrumb are the two that are looking at the notes.
   */
  readonly selectableCount = input<number | null>(null);

  /** After a rename or a deletion; a recolour leaves the panel open on the new swatch. */
  readonly finished = output<void>();
  readonly selectRequested = output<string>();

  private readonly folders = inject(FoldersStore);

  protected readonly colours = COLOURS;

  /** Two steps: the WebView blocks on a native `confirm()`. */
  protected readonly confirmingDelete = signal(false);

  private readonly renameInput = viewChild<ElementRef<HTMLInputElement>>('renameInput');

  focusName(): void {
    this.renameInput()?.nativeElement.focus();
  }

  protected submitRename(event: Event, name: string): void {
    event.preventDefault();
    if (!name.trim()) return;

    void this.folders.renameFolder(this.folder().id, name);
    this.finished.emit();
  }

  protected selectNotes(): void {
    this.selectRequested.emit(this.folder().id);
  }

  protected pickColour(colour: FolderColour): void {
    if (this.folder().colour === colour) return;

    void this.folders.recolourFolder(this.folder().id, colour);
  }

  /** ⚠️ No refuge to choose, unlike a space: the notes come out loose. */
  protected onDeleteClick(): void {
    if (!this.confirmingDelete()) {
      this.confirmingDelete.set(true);
      return;
    }
    void this.folders.deleteFolder(this.folder().id);
    this.finished.emit();
  }
}
