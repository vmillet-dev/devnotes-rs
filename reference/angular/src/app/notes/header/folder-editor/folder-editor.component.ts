import {
  ChangeDetectionStrategy,
  Component,
  DestroyRef,
  ElementRef,
  afterRenderEffect,
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
 * Rename, recolour, make a folder inside, delete — what a folder can be told to do, in one
 * panel so the switcher, the breadcrumb, the rail and the zone menu cannot drift apart. It
 * performs them itself; its host only hears `finished`, to close whatever it was opened from.
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
   * Opt-in rather than always drawn: the switcher lists folders you are **not** in, and
   * "select all the notes of that one over there" is a gesture with no visible result. The
   * zone menu and the breadcrumb are the two that are looking at the notes.
   */
  readonly selectableCount = input<number | null>(null);

  /** After a rename, a deletion or a sub-folder made; a recolour leaves the panel open. */
  readonly finished = output<void>();
  /** The sub-folder made, before `finished`: the rail opens its parent and marks it. */
  readonly created = output<Folder>();
  readonly selectRequested = output<string>();

  private readonly folders = inject(FoldersStore);
  private readonly destroyRef = inject(DestroyRef);

  protected readonly colours = COLOURS;

  /** Two steps: the WebView blocks on a native `confirm()`. */
  protected readonly confirmingDelete = signal(false);
  protected readonly nesting = signal(false);

  private readonly renameInput = viewChild<ElementRef<HTMLInputElement>>('renameInput');
  private readonly subfolderInput = viewChild<ElementRef<HTMLInputElement>>('subfolderInput');

  constructor() {
    afterRenderEffect(() => {
      if (this.nesting()) this.subfolderInput()?.nativeElement.focus();
    });
  }

  focusName(): void {
    this.renameInput()?.nativeElement.focus();
  }

  protected submitRename(event: Event, name: string): void {
    event.preventDefault();
    if (!name.trim()) return;

    void this.folders.renameFolder(this.folder().id, name);
    this.finished.emit();
  }

  /** A refused name leaves the field open to be corrected; the notifier has said why. */
  protected async submitSubfolder(event: Event, name: string): Promise<void> {
    event.preventDefault();
    if (!name.trim()) return;

    const created = await this.folders.createFolder(name, this.folder().id);
    if (created === null || this.destroyRef.destroyed) return;
    this.created.emit(created);
    this.finished.emit();
  }

  protected selectNotes(): void {
    this.selectRequested.emit(this.folder().id);
  }

  protected pickColour(colour: FolderColour): void {
    if (this.folder().colour === colour) return;

    void this.folders.recolourFolder(this.folder().id, colour);
  }

  /** No refuge to choose, unlike a space: its notes and sub-folders go up to its parent. */
  protected onDeleteClick(): void {
    if (!this.confirmingDelete()) {
      this.confirmingDelete.set(true);
      return;
    }
    void this.folders.deleteFolder(this.folder().id);
    this.finished.emit();
  }
}
