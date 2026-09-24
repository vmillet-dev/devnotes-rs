import {
  ChangeDetectionStrategy,
  Component,
  ElementRef,
  computed,
  effect,
  input,
  output,
  signal,
  viewChild,
} from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { Folder } from '@core/model/folder.model';
import { RAIL_WIDTH } from '@core/services/settings/app-settings.model';
import { Space } from '@core/model/space.model';
import { FolderEditorComponent } from '@notes/header/folder-editor/folder-editor.component';
import { SpaceEditorComponent } from '@notes/header/space-editor/space-editor.component';

/** A space and the folders drawn under it. */
interface SpaceNode {
  readonly space: Space;
  readonly folders: readonly Folder[];
}

/** What one arrow key moves the edge by: a nudge, not a jump. */
const RESIZE_STEP_PX = 16;

/**
 * The library as a tree: every space, its folders under it, the way an editor holds a
 * project. It replaces the two switchers while it is showing — two places to change
 * space is how a tree and a dropdown drift apart.
 *
 * ⚠️ Nothing here narrows the canvas itself. A row answers *where to go*; the page turns
 * that into a space and a folder, and the stores do the rest.
 */
@Component({
  selector: 'app-library-tree',
  imports: [TranslocoPipe, FolderEditorComponent, SpaceEditorComponent],
  // The width is the host's: the tree scrolls inside it and the edge is drawn against it.
  host: { '[style.width.px]': 'width()' },
  templateUrl: './library-tree.component.html',
  styleUrl: './library-tree.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class LibraryTreeComponent {
  readonly spaces = input.required<readonly Space[]>();
  /** Every folder in the library: the tree groups them, it does not ask twice. */
  readonly folders = input.required<readonly Folder[]>();
  /** `null` = "all spaces", a choice in its own right and not a waiting state. */
  readonly activeSpaceId = input.required<string | null>();
  readonly activeFolderId = input.required<string | null>();
  readonly width = input<number>(RAIL_WIDTH.default);

  readonly spaceChanged = output<string | null>();
  readonly spaceCreated = output<string>();
  readonly folderOpened = output<Folder>();
  readonly folderCreated = output<string>();
  readonly widthChanged = output<number>();

  /**
   * ⚠️ What the user *closed*, not what they opened: a library is worth showing whole,
   * and a set of opened ids would leave a fresh install looking empty.
   */
  private readonly collapsed = signal<ReadonlySet<string>>(new Set());

  protected readonly creatingSpace = signal(false);
  protected readonly creatingFolder = signal(false);
  protected readonly editedSpace = signal<string | null>(null);
  protected readonly editedFolder = signal<string | null>(null);

  protected readonly bounds = RAIL_WIDTH;

  private readonly spaceInput = viewChild<ElementRef<HTMLInputElement>>('spaceInput');
  private readonly folderInput = viewChild<ElementRef<HTMLInputElement>>('folderInput');
  private readonly spaceEditor = viewChild(SpaceEditorComponent);
  private readonly folderEditor = viewChild(FolderEditorComponent);

  protected readonly nodes = computed<readonly SpaceNode[]>(() =>
    this.spaces().map((space) => ({
      space,
      folders: this.folders().filter((folder) => folder.spaceId === space.id),
    })),
  );

  /** A space cannot be its own refuge: the cascade would take the notes after the move. */
  protected readonly moveTargets = computed<readonly Space[]>(() => {
    const edited = this.editedSpace();
    return edited === null ? [] : this.spaces().filter((space) => space.id !== edited);
  });

  constructor() {
    effect(() => {
      if (this.editedSpace() !== null) {
        this.spaceEditor()?.focusName();
      } else if (this.editedFolder() !== null) {
        this.folderEditor()?.focusName();
      } else if (this.creatingSpace()) {
        this.spaceInput()?.nativeElement.focus();
      } else if (this.creatingFolder()) {
        this.folderInput()?.nativeElement.focus();
      }
    });
  }

  /**
   * ⚠️ Pointer events, like every other drag in this application: HTML5 drag and drop
   * does not work in this WebView and cannot be turned on. The edge is a `separator`, so
   * the arrow keys move it too — the same rule the board's grips follow.
   */
  protected startResize(event: PointerEvent): void {
    if (event.button !== 0) return;

    event.preventDefault();
    const handle = event.target as HTMLElement;
    handle.setPointerCapture?.(event.pointerId);
  }

  protected onResizeMove(event: PointerEvent): void {
    const handle = event.target as HTMLElement;
    if (!handle.hasPointerCapture?.(event.pointerId)) return;

    // Measured from the rail's own left edge, so a window that is not at x=0 is no trap.
    const left = handle.parentElement?.getBoundingClientRect().left ?? 0;
    this.emitWidth(event.clientX - left);
  }

  protected endResize(event: PointerEvent): void {
    (event.target as HTMLElement).releasePointerCapture?.(event.pointerId);
  }

  protected onResizeKey(event: KeyboardEvent): void {
    const step =
      event.key === 'ArrowRight' ? RESIZE_STEP_PX : event.key === 'ArrowLeft' ? -RESIZE_STEP_PX : 0;
    if (step === 0) return;

    event.preventDefault();
    this.emitWidth(this.width() + step);
  }

  private emitWidth(width: number): void {
    const clamped = Math.min(RAIL_WIDTH.max, Math.max(RAIL_WIDTH.min, Math.round(width)));
    if (clamped !== this.width()) {
      this.widthChanged.emit(clamped);
    }
  }

  protected isExpanded(spaceId: string): boolean {
    return !this.collapsed().has(spaceId);
  }

  protected folderListId(spaceId: string): string {
    return `library-folders-${spaceId}`;
  }

  protected toggleExpanded(spaceId: string): void {
    this.collapsed.update((closed) => {
      const next = new Set(closed);
      if (!next.delete(spaceId)) {
        next.add(spaceId);
      }
      return next;
    });
  }

  /** Choosing a space opens it: a row that selects without showing what is inside is half a tree. */
  protected chooseSpace(id: string | null): void {
    this.closePanels();
    if (id !== null) {
      this.collapsed.update((closed) => {
        const next = new Set(closed);
        next.delete(id);
        return next;
      });
    }
    this.spaceChanged.emit(id);
  }

  protected chooseFolder(folder: Folder): void {
    this.closePanels();
    this.folderOpened.emit(folder);
  }

  protected startCreatingSpace(): void {
    this.closePanels();
    this.creatingSpace.set(true);
  }

  protected startCreatingFolder(): void {
    this.closePanels();
    this.creatingFolder.set(true);
  }

  protected editSpace(id: string): void {
    const current = this.editedSpace();
    this.closePanels();
    this.editedSpace.set(current === id ? null : id);
  }

  protected editFolder(id: string): void {
    const current = this.editedFolder();
    this.closePanels();
    this.editedFolder.set(current === id ? null : id);
  }

  /** `submit` and not `click`: the form then also answers Enter. */
  protected submitNewSpace(event: Event, name: string): void {
    event.preventDefault();
    if (!name.trim()) return;

    this.creatingSpace.set(false);
    this.spaceCreated.emit(name);
  }

  protected submitNewFolder(event: Event, name: string): void {
    event.preventDefault();
    if (!name.trim()) return;

    this.creatingFolder.set(false);
    this.folderCreated.emit(name);
  }

  /** One panel at a time: two open forms in a rail this narrow read as one. */
  protected closePanels(): void {
    this.creatingSpace.set(false);
    this.creatingFolder.set(false);
    this.editedSpace.set(null);
    this.editedFolder.set(null);
  }
}
