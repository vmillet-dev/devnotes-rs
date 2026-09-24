import { ChangeDetectionStrategy, Component, DestroyRef, inject } from '@angular/core';
import { AppEventsService, GlobalAction } from '@core/ipc/app-events.service';
import { NotesRevision } from '@core/state/notes-revision';
import { NotesStore } from '@core/state/notes.store';
import { PaletteStore } from '@core/state/palette.store';
import { SampleNotesService } from '@core/state/sample-notes.service';
import { SpacesStore } from '@core/state/spaces.store';
import { CanvasKeyboardDirective } from '@shared/directives/canvas-keyboard.directive';
import { NotesCanvasComponent } from './canvas/notes-canvas.component';
import { NotesHeaderComponent } from './header/notes-header.component';
import { NotesOverlaysComponent } from './overlays/notes-overlays.component';
import { NotesSidebarComponent } from './sidebar/notes-sidebar.component';

/**
 * A layout of four zones, each a container injecting what it draws. The page keeps what is
 * page-wide: the actions the native side asks for, the first-launch seeding, and the
 * keyboard of the canvas.
 */
@Component({
  selector: 'app-notes-page',
  imports: [NotesSidebarComponent, NotesHeaderComponent, NotesCanvasComponent, NotesOverlaysComponent],
  hostDirectives: [CanvasKeyboardDirective],
  templateUrl: './notes-page.component.html',
  styleUrl: './notes-page.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class NotesPageComponent {
  private readonly store = inject(NotesStore);
  private readonly palette = inject(PaletteStore);
  private readonly spaces = inject(SpacesStore);
  private readonly samples = inject(SampleNotesService);
  private readonly revision = inject(NotesRevision);

  constructor() {
    const events = inject(AppEventsService);
    inject(DestroyRef).onDestroy(events.on((action) => this.runGlobalAction(action)));

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
}
