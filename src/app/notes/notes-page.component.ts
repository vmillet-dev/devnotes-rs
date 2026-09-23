import { ChangeDetectionStrategy, Component, DestroyRef, computed, inject, signal } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { AppEventsService, GlobalAction } from '@core/ipc/app-events.service';
import { DialogStack } from '@shared/layout/dialog/dialog-stack';
import { AttachmentsStore } from '@core/state/attachments.store';
import { LibraryStore } from '@core/state/library.store';
import { NoteSelectionStore } from '@core/state/note-selection.store';
import { BoardStore } from '@core/state/board.store';
import { FoldersStore } from '@core/state/folders.store';
import { NotesQueryStore } from '@core/state/notes-query.store';
import { NotesRevision } from '@core/state/notes-revision';
import { NoteFilter, NotesStore } from '@core/state/notes.store';
import { PlaceholderFillStore } from '@core/state/placeholder-fill.store';
import { SampleNotesService } from '@core/state/sample-notes.service';
import { HelpStore } from '@core/services/help/help.store';
import { SettingsStore } from '@core/services/settings/settings.store';
import { PaletteStore } from '@core/state/palette.store';
import { SpacesStore } from '@core/state/spaces.store';
import { TagsStore } from '@core/state/tags.store';
import { TrashStore } from '@core/state/trash.store';
import { CanvasKeyboardDirective } from '@shared/directives/canvas-keyboard.directive';
import { BoardFrame } from '@core/model/board.model';
import { Folder } from '@core/model/folder.model';
import { Note } from '@core/model/note.model';
import { BoardComponent, CardDrop } from './canvas/board/board.component';
import { LibraryTreeComponent } from './sidebar/library-tree/library-tree.component';
import { FolderNamePromptComponent } from './overlays/folder-name-prompt/folder-name-prompt.component';
import { FacetsPanelComponent } from './header/facets-panel/facets-panel.component';
import { FolderRecolouring, FolderRenaming } from './header/folder-editor/folder-editor.component';
import { FolderBreadcrumbComponent } from './header/folder-breadcrumb/folder-breadcrumb.component';
import { FolderSwitcherComponent } from './header/folder-switcher/folder-switcher.component';
import { NewNoteButtonComponent } from './header/new-note-button/new-note-button.component';
import { NoteActivation } from './canvas/note-card/note-card.component';
import { NoteEditorOverlayComponent } from './overlays/note-editor-overlay/note-editor-overlay.component';
import { NoteSectionComponent } from './canvas/note-section/note-section.component';
import { PlaceholderFormComponent } from './overlays/placeholder-form/placeholder-form.component';
import { ImageLightboxComponent } from './overlays/image-lightbox/image-lightbox.component';
import { QuickPaletteComponent } from './overlays/quick-palette/quick-palette.component';
import { SearchBoxComponent } from './header/search-box/search-box.component';
import { SelectionBarComponent } from './header/selection-bar/selection-bar.component';
import { SpaceDeletion, SpaceRenaming } from './header/space-editor/space-editor.component';
import { SpaceSwitcherComponent } from './header/space-switcher/space-switcher.component';
import { TagManagerComponent } from './overlays/tag-manager/tag-manager.component';
import { ViewSwitchComponent } from './header/view-switch/view-switch.component';
import {
  Segment,
  SegmentedChoiceComponent,
} from '@shared/controls/segmented-choice/segmented-choice.component';
import { IconComponent } from '@shared/icon/icon.component';
import { TrashPanelComponent } from './overlays/trash-panel/trash-panel.component';
import { UndoBarComponent } from './overlays/undo-bar/undo-bar.component';

/** Three states of one thing, so one control rather than three chips. */
const QUICK_FILTERS: readonly Segment[] = (['all', 'pinned', 'untriaged'] as const).map((key) => ({
  id: key,
  labelKey: `filters.${key}`,
}));

@Component({
  selector: 'app-notes-page',
  imports: [
    SpaceSwitcherComponent,
    SearchBoxComponent,
    FacetsPanelComponent,
    SegmentedChoiceComponent,
    IconComponent,
    FolderSwitcherComponent,
    FolderBreadcrumbComponent,
    LibraryTreeComponent,
    ViewSwitchComponent,
    BoardComponent,
    FolderNamePromptComponent,
    NewNoteButtonComponent,
    SelectionBarComponent,
    NoteSectionComponent,
    NoteEditorOverlayComponent,
    QuickPaletteComponent,
    ImageLightboxComponent,
    PlaceholderFormComponent,
    TrashPanelComponent,
    TagManagerComponent,
    UndoBarComponent,
    TranslocoPipe,
  ],
  hostDirectives: [CanvasKeyboardDirective],
  templateUrl: './notes-page.component.html',
  styleUrl: './notes-page.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class NotesPageComponent {
  protected readonly canvas = inject(NotesQueryStore);
  protected readonly selection = inject(NoteSelectionStore);
  protected readonly store = inject(NotesStore);
  protected readonly spaces = inject(SpacesStore);
  protected readonly folders = inject(FoldersStore);
  protected readonly board = inject(BoardStore);
  protected readonly palette = inject(PaletteStore);
  protected readonly trash = inject(TrashStore);
  protected readonly tags = inject(TagsStore);
  protected readonly library = inject(LibraryStore);
  protected readonly attachments = inject(AttachmentsStore);
  protected readonly fill = inject(PlaceholderFillStore);
  protected readonly settings = inject(SettingsStore);
  /** The guide, opened at the chapter about whatever is empty on screen. */
  protected readonly help = inject(HelpStore);

  /**
   * ⚠️ The disclosure is the page's and not the panel's: the trigger lives in the topbar
   * and the panel below it, and a component cannot be in two rows at once.
   */
  protected readonly facetsExpanded = signal(false);

  protected readonly facetCount = computed(
    () => this.canvas.selectedTags().size + this.canvas.selectedLanguages().size,
  );

  /** ⚠️ Forced open by a selection: a filter nobody can see is a filter nobody can undo. */
  protected readonly facetsOpen = computed(() => this.facetsExpanded() || this.facetCount() > 0);

  protected readonly quickFilters = QUICK_FILTERS;

  /** The segmented control speaks in strings; the store speaks in `NoteFilter`. */
  protected onQuickFilter(key: string): void {
    this.canvas.setFilter(key as NoteFilter);
  }

  private readonly samples = inject(SampleNotesService);
  private readonly revision = inject(NotesRevision);
  private readonly dialogs = inject(DialogStack);

  /** Asked of `DialogStack` rather than of each store in turn. */
  protected readonly searchShortcutEnabled = computed(() => !this.dialogs.hasOpenDialog());

  /**
   * The legend names keys that act on cards, so it stands down whenever the region holds
   * something else — loading, an error, or the empty state, which already owns the screen
   * and offers the one thing to do from there.
   */
  protected readonly showsKeyboardHint = computed(() => {
    if (this.board.isShowing()) {
      return !this.board.isLoading() && this.board.loadError() === undefined;
    }
    return !this.canvas.isLoading() && this.canvas.loadError() === undefined && !this.canvas.hasNoResults();
  });

  constructor() {
    const events = inject(AppEventsService);
    const destroyRef = inject(DestroyRef);

    destroyRef.onDestroy(events.on((action) => this.runGlobalAction(action)));

    // A fresh installation has no space, so not even a creatable note.
    void this.seedSamples();
  }

  /**
   * The native side says what was wanted; creating the note stays here. No `default`:
   * the switch is exhaustive over a generated union, so a variant added in Rust stops
   * this compiling until it is handled.
   */
  private runGlobalAction(action: GlobalAction): void {
    switch (action) {
      case 'capture':
        void this.store.captureFromClipboard();
        break;
      case 'new-note':
        this.store.createNote();
        break;
      case 'palette':
        void this.palette.open();
        break;
    }
  }

  /**
   * The spaces reload afterwards — they had already read an empty database.
   *
   * ⚠️ And the seeded space is selected: with exactly one, "all spaces" is a distinction
   * without a difference, and it is the state in which the board cannot be shown at all —
   * a first launch would hide the feature behind a disabled button.
   */
  private async seedSamples(): Promise<void> {
    const seeded = await this.samples.seedIfFirstRun();
    if (!seeded) return;

    this.spaces.reload();
    this.spaces.selectSpace(seeded.id);
    this.revision.bump();
  }

  protected toggleLibraryRail(): void {
    this.settings.showLibraryRail.write(!this.settings.showLibraryRail());
  }

  /** Choosing a space leaves whatever folder was open: the row means the space itself. */
  protected onSpaceChosen(id: string | null): void {
    this.folders.selectFolder(null);
    this.spaces.selectSpace(id);
  }

  /** ⚠️ The space first: a folder is resolved against the active space's folders. */
  protected onFolderOpened(folder: Folder): void {
    this.spaces.selectSpace(folder.spaceId);
    this.folders.selectFolder(folder.id);
  }

  protected onSpaceRenamed({ id, name }: SpaceRenaming): void {
    // Nothing to reload: a note carries only the `spaceId`, never the name.
    void this.spaces.renameSpace(id, name);
  }

  protected onSpaceDeleted({ id, targetSpaceId }: SpaceDeletion): void {
    void this.spaces.deleteSpace(id, targetSpaceId);
  }

  protected onFolderRenamed({ id, name }: FolderRenaming): void {
    void this.folders.renameFolder(id, name);
  }

  protected onFolderRecoloured({ id, colour }: FolderRecolouring): void {
    void this.folders.recolourFolder(id, colour);
  }

  /** Where a band was drawn, held until it has been given a name. */
  protected readonly pendingZone = signal<BoardFrame | null>(null);

  protected onCardDropped({ noteId, folderId, position }: CardDrop): void {
    void this.board.dropCard(noteId, folderId, position);
  }

  /** Deleting from here goes back to the board, with the notes now loose on it. */
  protected onFolderDeleted(id: string): void {
    void this.folders.deleteFolder(id);
  }

  protected onZoneDrawn(frame: BoardFrame): void {
    this.pendingZone.set(frame);
  }

  protected async onZoneNamed(frame: BoardFrame, name: string): Promise<void> {
    this.pendingZone.set(null);
    await this.board.createZone(name, frame);
    this.folders.reload();
  }

  protected onNoteActivated({ noteId, toggleChecked, extendRange }: NoteActivation): void {
    if (extendRange) {
      this.selection.checkRangeTo(noteId);
      return;
    }
    if (toggleChecked) {
      this.selection.toggleChecked(noteId);
      this.selection.focusNote(noteId);
      return;
    }
    this.store.openNote(noteId);
  }

  protected onCopySelection(): void {
    void this.library.copyAsMarkdown(this.selection.checkedNoteIds());
  }

  protected onRestore(id: string): void {
    void this.trash.restore(id);
  }

  protected onRenameTag(into: string): void {
    void this.tags.proposeRename(into);
  }

  protected onDeleteTags(): void {
    void this.tags.proposeDelete();
  }

  /** ⚠️ The note and not its id: the canvas filters may hide it, and often do (#280). */
  protected onPaletteOpen(note: Note): void {
    this.palette.close();
    this.store.openNote(note);
  }

  /** `PaletteStore` does not know `NotesStore`; the other way round would be a cycle. */
  protected async onPaletteChosen(): Promise<void> {
    const content = this.palette.takeNewNoteContent();
    if (content !== null) {
      await this.store.createWithContent(content);
      return;
    }

    await this.palette.chooseHighlighted();
  }
}
