import {
  ChangeDetectionStrategy,
  Component,
  ElementRef,
  input,
  output,
  signal,
  viewChild,
} from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { Folder, FolderColour } from '@core/model/folder.model';

export interface FolderRenaming {
  readonly id: string;
  readonly name: string;
}

export interface FolderRecolouring {
  readonly id: string;
  readonly colour: FolderColour;
}

/** The palette the back end assigns from; the order is `FolderColour::ALL`. */
const COLOURS: readonly FolderColour[] = ['blue', 'amber', 'purple', 'green', 'red'];

/**
 * Rename, recolour, delete — the three things a folder can be told to do, in one panel so
 * the switcher, the breadcrumb and the zone menu on the board cannot drift apart.
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

  readonly renamed = output<FolderRenaming>();
  readonly recoloured = output<FolderRecolouring>();
  readonly deleted = output<string>();
  readonly selectRequested = output<string>();

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

    this.renamed.emit({ id: this.folder().id, name });
  }

  protected selectNotes(): void {
    this.selectRequested.emit(this.folder().id);
  }

  protected pickColour(colour: FolderColour): void {
    if (this.folder().colour === colour) return;

    this.recoloured.emit({ id: this.folder().id, colour });
  }

  /** ⚠️ No refuge to choose, unlike a space: the notes come out loose. */
  protected onDeleteClick(): void {
    if (!this.confirmingDelete()) {
      this.confirmingDelete.set(true);
      return;
    }
    this.deleted.emit(this.folder().id);
  }
}
