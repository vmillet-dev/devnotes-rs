import { ChangeDetectionStrategy, Component, computed, inject, signal } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { BoardStore } from '@core/state/board.store';
import { FoldersStore } from '@core/state/folders.store';
import { NoteSelectionStore } from '@core/state/note-selection.store';
import { NotesQueryStore } from '@core/state/notes-query.store';
import { NoteFilter, NotesStore } from '@core/state/notes.store';
import { SpacesStore } from '@core/state/spaces.store';
import { TagsStore } from '@core/state/tags.store';
import { TrashStore } from '@core/state/trash.store';
import { SettingsStore } from '@core/services/settings/settings.store';
import { ShortcutBindingsStore } from '@core/services/shortcuts/shortcut-bindings.store';
import { acceleratorKeys } from '@core/services/shortcuts/shortcut.model';
import { DialogStack } from '@shared/layout/dialog/dialog-stack';
import {
  Segment,
  SegmentedChoiceComponent,
} from '@shared/controls/segmented-choice/segmented-choice.component';
import { IconComponent } from '@shared/icon/icon.component';
import { FacetsPanelComponent } from './facets-panel/facets-panel.component';
import { FolderBreadcrumbComponent } from './folder-breadcrumb/folder-breadcrumb.component';
import { FolderSwitcherComponent } from './folder-switcher/folder-switcher.component';
import { NewNoteButtonComponent } from './new-note-button/new-note-button.component';
import { SearchBoxComponent } from './search-box/search-box.component';
import { SpaceSwitcherComponent } from './space-switcher/space-switcher.component';
import { ViewSwitchComponent } from './view-switch/view-switch.component';
import { CREATE_ACTIONS } from '../canvas-keys';

/** Three states of one thing, so one control rather than three chips. */
const QUICK_FILTERS: readonly Segment[] = (['all', 'pinned', 'untriaged'] as const).map((key) => ({
  id: key,
  labelKey: `filters.${key}`,
}));

/** Above the canvas: the topbar, and the facets it discloses. */
@Component({
  selector: 'app-notes-header',
  imports: [
    SpaceSwitcherComponent,
    SearchBoxComponent,
    FacetsPanelComponent,
    SegmentedChoiceComponent,
    IconComponent,
    FolderSwitcherComponent,
    FolderBreadcrumbComponent,
    ViewSwitchComponent,
    NewNoteButtonComponent,
    TranslocoPipe,
  ],
  templateUrl: './notes-header.component.html',
  styleUrl: './notes-header.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class NotesHeaderComponent {
  protected readonly canvas = inject(NotesQueryStore);
  protected readonly selection = inject(NoteSelectionStore);
  protected readonly store = inject(NotesStore);
  protected readonly spaces = inject(SpacesStore);
  protected readonly folders = inject(FoldersStore);
  protected readonly board = inject(BoardStore);
  protected readonly trash = inject(TrashStore);
  protected readonly tags = inject(TagsStore);
  protected readonly settings = inject(SettingsStore);
  private readonly dialogs = inject(DialogStack);
  private readonly bindings = inject(ShortcutBindingsStore);

  protected readonly quickFilters = QUICK_FILTERS;

  /**
   * The disclosure is the header's and not the panel's: the trigger lives in the topbar
   * and the panel below it, and a component cannot be in two rows at once.
   */
  protected readonly facetsExpanded = signal(false);

  protected readonly facetCount = computed(
    () =>
      this.canvas.selectedTags().size +
      this.canvas.selectedLanguages().size +
      this.canvas.selectedKinds().size,
  );

  /** Forced open by a selection: a filter nobody can see is a filter nobody can undo. */
  protected readonly facetsOpen = computed(() => this.facetsExpanded() || this.facetCount() > 0);

  /** Read from the bindings, so a key moved in the preferences is the one the menu names. */
  protected readonly createShortcuts = computed(() => ({
    snippet: acceleratorKeys(this.bindings.binding(CREATE_ACTIONS.snippet)),
    note: acceleratorKeys(this.bindings.binding(CREATE_ACTIONS.note)),
    checklist: acceleratorKeys(this.bindings.binding(CREATE_ACTIONS.checklist)),
  }));

  /** Asked of `DialogStack` rather than of each store in turn. */
  protected readonly searchShortcutEnabled = computed(() => !this.dialogs.hasOpenDialog());

  protected toggleLibraryRail(): void {
    this.settings.showLibraryRail.write(!this.settings.showLibraryRail());
  }

  /** The segmented control speaks in strings; the store speaks in `NoteFilter`. */
  protected onQuickFilter(key: string): void {
    this.canvas.setFilter(key as NoteFilter);
  }
}
