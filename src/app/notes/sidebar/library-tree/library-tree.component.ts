import {
  ChangeDetectionStrategy,
  Component,
  DestroyRef,
  ElementRef,
  afterRenderEffect,
  computed,
  inject,
  input,
  output,
  signal,
  viewChild,
} from '@angular/core';
import { NgTemplateOutlet } from '@angular/common';
import { TranslocoPipe } from '@jsverse/transloco';
import { Folder, canMoveFolder, childrenOf } from '@core/model/folder.model';
import { RAIL_WIDTH } from '@core/services/settings/app-settings.model';
import { Space } from '@core/model/space.model';
import { FoldersStore } from '@core/state/folders.store';
import { UndoStore } from '@core/state/undo.store';
import { FolderEditorComponent } from '@notes/header/folder-editor/folder-editor.component';
import { SpaceEditorComponent } from '@notes/header/space-editor/space-editor.component';
import { RailFrameComponent } from '@shared/layout/rail-frame/rail-frame.component';
import { RailMarks } from '@shared/rail/rail-marks';
import { RowDrag, railRowAt } from '@shared/rail/row-drag';

/** Where a folder dragged in the rail would go: under a folder, or at the top of its space. */
interface FolderDrop {
  /** The row under the pointer: a folder's id, or its space's. */
  readonly id: string;
  readonly parentId: string | null;
  /** Drawn in red and said: into itself, or into another space. */
  readonly refused: boolean;
}

/**
 * The library as a tree: every space, its folders under it and theirs under them, the way an
 * editor holds a project. It replaces the two switchers while it is showing — two places to
 * change space is how a tree and a dropdown drift apart.
 *
 * Nothing here narrows the canvas itself. A row answers *where to go*; the page turns that
 * into a space and a folder, and the stores do the rest. Moving a folder is the rail's own:
 * dragged onto another row, or `Alt` and the arrows, marked and said as the HTTP rail does.
 */
@Component({
  selector: 'app-library-tree',
  imports: [FolderEditorComponent, NgTemplateOutlet, RailFrameComponent, SpaceEditorComponent, TranslocoPipe],
  templateUrl: './library-tree.component.html',
  styleUrl: './library-tree.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class LibraryTreeComponent {
  private readonly store = inject(FoldersStore);
  private readonly undo = inject(UndoStore);
  private readonly host = inject<ElementRef<HTMLElement>>(ElementRef);

  readonly spaces = input.required<readonly Space[]>();
  /** Every folder in the library: the tree groups them, it does not ask twice. */
  readonly folders = input.required<readonly Folder[]>();
  /** `null` = "all spaces", a choice in its own right and not a waiting state. */
  readonly activeSpaceId = input.required<string | null>();
  readonly activeFolderId = input.required<string | null>();
  readonly width = input<number>(RAIL_WIDTH.default);
  /** The row a card dragged on the board would be filed into: lit as a folder dragged here is. */
  readonly cardTarget = input<string | null>(null);

  readonly spaceChanged = output<string | null>();
  readonly spaceCreated = output<string>();
  readonly folderOpened = output<Folder>();
  readonly folderCreated = output<string>();
  readonly widthChanged = output<number>();

  /**
   * What the user *closed*, not what they opened: a library is worth showing whole,
   * and a set of opened ids would leave a fresh install looking empty.
   */
  private readonly collapsed = signal<ReadonlySet<string>>(new Set());

  protected readonly creatingSpace = signal(false);
  protected readonly creatingFolder = signal(false);
  protected readonly editedSpace = signal<string | null>(null);
  protected readonly editedFolder = signal<string | null>(null);

  protected readonly bounds = RAIL_WIDTH;
  protected readonly drag = new RowDrag<Folder, FolderDrop>((x, y, folder) => this.dropAt(x, y, folder));
  protected readonly marks = new RailMarks('sidebar.moved');

  private readonly spaceInput = viewChild<ElementRef<HTMLInputElement>>('spaceInput');
  private readonly folderInput = viewChild<ElementRef<HTMLInputElement>>('folderInput');
  private readonly spaceEditor = viewChild(SpaceEditorComponent);
  private readonly folderEditor = viewChild(FolderEditorComponent);

  /** Each level's folders by name, keyed by space and parent. */
  private readonly levels = computed(() => {
    const levels = new Map<string, readonly Folder[]>();
    const keys = new Set(this.folders().map((folder) => levelKey(folder.spaceId, folder.parentId)));
    for (const key of keys) {
      const [spaceId, parentId] = key.split('\u0000');
      const inSpace = this.folders().filter((folder) => folder.spaceId === spaceId);
      levels.set(key, childrenOf(inSpace, parentId || null));
    }
    return levels;
  });

  /** A space cannot be its own refuge: the cascade would take the notes after the move. */
  protected readonly moveTargets = computed<readonly Space[]>(() => {
    const edited = this.editedSpace();
    return edited === null ? [] : this.spaces().filter((space) => space.id !== edited);
  });

  constructor() {
    inject(DestroyRef).onDestroy(() => this.marks.dispose());
    afterRenderEffect(() => {
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

  protected childrenOf(spaceId: string, parentId: string | null): readonly Folder[] {
    return this.levels().get(levelKey(spaceId, parentId)) ?? [];
  }

  protected hasChildren(folder: Folder): boolean {
    return this.childrenOf(folder.spaceId, folder.id).length > 0;
  }

  protected isExpanded(id: string): boolean {
    return !this.collapsed().has(id);
  }

  protected folderListId(id: string): string {
    return `library-folders-${id}`;
  }

  protected toggleExpanded(id: string): void {
    this.collapsed.update((closed) => {
      const next = new Set(closed);
      if (!next.delete(id)) {
        next.add(id);
      }
      return next;
    });
  }

  /** Lit while a folder or a card would land in it. */
  protected isDropTarget(id: string): boolean {
    const target = this.drag.target();
    return (target?.id === id && !target.refused) || this.cardTarget() === id;
  }

  protected isRefused(id: string): boolean {
    const target = this.drag.target();
    return target?.id === id && target.refused;
  }

  /** Choosing a space opens it: a row that selects without showing what is inside is half a tree. */
  protected chooseSpace(id: string | null): void {
    this.closePanels();
    if (id !== null) this.reveal(id);
    this.spaceChanged.emit(id);
  }

  protected chooseFolder(event: MouseEvent, folder: Folder): void {
    if (this.drag.endsADrag(event)) return;
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

  protected async endDrag(event: PointerEvent): Promise<void> {
    const ended = this.drag.end(event);
    const target = ended?.target;
    if (!ended || !target) return;

    if (target.refused) {
      this.marks.say('refused.there', { name: ended.row.name });
      return;
    }
    await this.move(ended.row, target.parentId);
  }

  /**
   * `Alt+←` up a level, `Alt+→` into the folder above it among its siblings — there is no order
   * of siblings to change, so no `Alt+↑/↓`.
   */
  protected async onFolderKeydown(event: KeyboardEvent, folder: Folder): Promise<void> {
    if (!event.altKey || event.ctrlKey || event.metaKey) return;
    if (event.key !== 'ArrowLeft' && event.key !== 'ArrowRight') return;
    event.preventDefault();

    if (event.key === 'ArrowLeft') {
      if (folder.parentId === null) {
        this.marks.say('refused.atTop', { name: folder.name });
        return;
      }
      const parent = this.folders().find((candidate) => candidate.id === folder.parentId);
      await this.move(folder, parent?.parentId ?? null);
    } else {
      const siblings = this.childrenOf(folder.spaceId, folder.parentId);
      const above = siblings[siblings.findIndex((sibling) => sibling.id === folder.id) - 1];
      if (above === undefined) {
        this.marks.say('refused.noFolderAbove', { name: folder.name });
        return;
      }
      await this.move(folder, above.id);
    }
    this.focusRow(folder.id);
  }

  /** Recorded for the undo, marked on the row and on what now holds it, and said. */
  private async move(folder: Folder, parentId: string | null): Promise<void> {
    const was = await this.store.moveFolder(folder.id, parentId);
    if (was === null) return;

    const parent = parentId === null ? null : this.folders().find((candidate) => candidate.id === parentId);
    const space = this.spaces().find((candidate) => candidate.id === folder.spaceId);
    this.undo.record({ kind: 'folderMove', was, name: folder.name, into: parent?.name ?? null, count: 1 });
    this.reveal(parentId ?? folder.spaceId);
    this.marks.mark({ id: folder.id, parentId: parentId ?? folder.spaceId });
    this.marks.say(parent ? 'into' : 'top', { name: folder.name, other: parent?.name ?? space?.name ?? '' });
  }

  /** Where a folder would land at a point: under the folder there, or at the top of its space. */
  private dropAt(x: number, y: number, dragged: Folder): FolderDrop | null {
    const element = railRowAt(this.host.nativeElement.ownerDocument, x, y);
    const id = element?.dataset['id'];
    const kind = element?.dataset['kind'];
    if (id === undefined || (kind !== 'folder' && kind !== 'space')) return null;

    if (kind === 'space') {
      if (id !== dragged.spaceId) return { id, parentId: null, refused: true };
      return dragged.parentId === null ? null : { id, parentId: null, refused: false };
    }
    if (id === dragged.parentId) return null;
    return { id, parentId: id, refused: !canMoveFolder(this.folders(), dragged.id, id) };
  }

  /** Opens what holds a row, so a folder moved there stays in sight. */
  private reveal(id: string): void {
    this.collapsed.update((closed) => {
      if (!closed.has(id)) return closed;
      const next = new Set(closed);
      next.delete(id);
      return next;
    });
  }

  private focusRow(id: string): void {
    requestAnimationFrame(() => {
      const name = this.host.nativeElement.querySelector<HTMLElement>(
        `[data-rail-row][data-id="${id}"] .node-name`,
      );
      name?.focus();
      // Absent from jsdom, and from nothing else this runs on.
      name?.scrollIntoView?.({ block: 'nearest' });
    });
  }
}

function levelKey(spaceId: string, parentId: string | null): string {
  return `${spaceId}\u0000${parentId ?? ''}`;
}
