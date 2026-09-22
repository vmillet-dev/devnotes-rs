import { ChangeDetectionStrategy, Component, Signal, computed, inject, signal } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { ClockService } from '@core/services/time/clock.service';
import { AppWindowService } from '@core/services/window/app-window.service';
import { LibraryStore } from '@core/state/library.store';
import { NoteSelectionStore } from '@core/state/note-selection.store';
import { SpacesStore } from '@core/state/spaces.store';
import { MenuPanelDirective } from '@shared/directives/menu-panel.directive';
import { MenuTriggerDirective } from '@shared/directives/menu-trigger.directive';
import { LibrariesDialogComponent } from '@titlebar/file-menu/libraries-dialog/libraries-dialog.component';
import { SettingsDialogComponent } from '@titlebar/file-menu/settings-dialog/settings-dialog.component';

/** `disabled` is a signal: a value frozen at construction would stop matching the screen. */
interface FileMenuEntry {
  readonly id: string;
  readonly labelKey: string;
  readonly disabled?: Signal<boolean>;
  readonly run: () => void;
}

@Component({
  selector: 'app-file-menu',
  imports: [TranslocoPipe, MenuPanelDirective, LibrariesDialogComponent, SettingsDialogComponent],
  hostDirectives: [MenuTriggerDirective],
  templateUrl: './file-menu.component.html',
  styleUrl: './file-menu.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class FileMenuComponent {
  private readonly window = inject(AppWindowService);
  private readonly library = inject(LibraryStore);
  private readonly selection = inject(NoteSelectionStore);
  private readonly spaces = inject(SpacesStore);
  private readonly clock = inject(ClockService);

  protected readonly menu = inject(MenuTriggerDirective);

  /** Quitting closes the application for good: a second click confirms it. */
  protected readonly confirmingQuit = signal(false);

  protected readonly settingsOpen = signal(false);
  protected readonly librariesOpen = signal(false);

  private readonly nothingChecked = computed(() => !this.selection.hasSelection());

  protected readonly entries: readonly FileMenuEntry[] = [
    { id: 'import', labelKey: 'file.import', run: () => void this.onImport() },
    {
      id: 'exportAll',
      labelKey: 'file.exportAll',
      run: () => void this.library.export(null, this.clock.now()),
    },
    {
      id: 'exportSpace',
      labelKey: 'file.exportSpace',
      disabled: computed(() => this.spaces.activeSpaceId() === null),
      run: () => void this.library.export(this.spaces.activeSpaceId(), this.clock.now()),
    },
    {
      id: 'exportSelection',
      labelKey: 'file.exportSelection',
      disabled: this.nothingChecked,
      run: () => void this.library.exportSelection(this.checked(), this.clock.now()),
    },
    {
      id: 'copyMarkdown',
      labelKey: 'file.copyMarkdown',
      disabled: this.nothingChecked,
      run: () => void this.library.copyAsMarkdown(this.checked()),
    },
  ];

  constructor() {
    this.menu.escaped.subscribe(() => this.menu.close());
    this.menu.closed.subscribe(() => this.confirmingQuit.set(false));
  }

  /** The report shows under the titlebar: the menu closes on the action. */
  protected run(entry: FileMenuEntry): void {
    if (entry.disabled?.()) return;

    entry.run();
    this.menu.close(false);
  }

  /** The preferences act on the application and not a tool, so the menu always offers them. */
  protected openLibraries(): void {
    this.librariesOpen.set(true);
    // No focus restored: the modal opening takes it itself.
    this.menu.close(false);
  }

  protected closeLibraries(): void {
    this.librariesOpen.set(false);
    this.menu.focusAnchor();
  }

  protected openSettings(): void {
    this.settingsOpen.set(true);
    // No focus restored: the modal opening takes it itself.
    this.menu.close(false);
  }

  /** The modal's focus trap would hand back to the menu entry, destroyed since. */
  protected closeSettings(): void {
    this.settingsOpen.set(false);
    this.menu.focusAnchor();
  }

  protected onQuit(): void {
    if (!this.confirmingQuit()) {
      this.confirmingQuit.set(true);
      return;
    }
    void this.window.quit();
  }

  /** Only the spaces reload: the canvas follows `NotesRevision`, which the library bumps. */
  private async onImport(): Promise<void> {
    if (await this.library.import()) {
      this.spaces.reload();
    }
  }

  private checked(): readonly string[] {
    return this.selection.checkedNoteIds();
  }
}
