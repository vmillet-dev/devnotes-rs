import {
  ChangeDetectionStrategy,
  Component,
  ElementRef,
  computed,
  inject,
  input,
  linkedSignal,
  output,
  signal,
} from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { Folder } from '@core/model/folder.model';
import { Space } from '@core/model/space.model';
import { MenuPanelDirective } from '@shared/directives/menu-panel.directive';
import { MenuTriggerDirective } from '@shared/directives/menu-trigger.directive';

@Component({
  selector: 'app-note-card-menu',
  imports: [TranslocoPipe, MenuPanelDirective],
  hostDirectives: [MenuTriggerDirective],
  templateUrl: './note-card-menu.component.html',
  styleUrl: './note-card-menu.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class NoteCardMenuComponent {
  /** An already displayable title: the card resolved the placeholder label. */
  readonly noteTitle = input.required<string>();
  readonly spaces = input.required<readonly Space[]>();
  readonly currentSpaceId = input.required<string>();
  /** The **current** space's folders: a folder belongs to one space. */
  readonly folders = input<readonly Folder[]>([]);
  readonly currentFolderId = input<string | null>(null);
  readonly pinned = input(false);

  readonly opened = output<void>();
  readonly pinToggled = output<void>();
  readonly copyRequested = output<void>();
  readonly duplicateRequested = output<void>();
  readonly fileRequested = output<string | null>();
  readonly moveRequested = output<string>();
  readonly deleteRequested = output<void>();

  protected readonly menu = inject(MenuTriggerDirective);
  private readonly host = inject<ElementRef<HTMLElement>>(ElementRef);

  /**
   * Where the panel is drawn, in viewport coordinates.
   *
   * `position: fixed`, measured, rather than `absolute` against the trigger. The card
   * sits inside the canvas, which scrolls — so an absolutely placed panel is clipped by
   * that box, and this menu became tall enough for it to matter the day it became the
   * complete one: it lost its last entry, which is the delete. Fixed escapes the clip, and
   * the flip is what keeps it on screen at either end of the canvas.
   */
  protected readonly panelStyle = signal<Record<string, string>>({});

  /** The gap the panel keeps from the window edge, and from its own trigger. */
  private static readonly MARGIN = 8;

  /** Two steps: the WebView blocks everything during a native `confirm()`. */
  protected readonly confirmingDelete = linkedSignal({
    source: this.menu.open,
    computation: () => false,
  });

  protected readonly moveTargets = computed<readonly Space[]>(() =>
    this.spaces().filter((space) => space.id !== this.currentSpaceId()),
  );

  /** Where it already is is not somewhere to file it. */
  protected readonly fileTargets = computed<readonly Folder[]>(() =>
    this.folders().filter((folder) => folder.id !== this.currentFolderId()),
  );

  protected toggle(event: MouseEvent): void {
    // The whole card is an opening button: without this, a click on the ⋯ bubbles up
    // and opens the editor along with the menu.
    event.stopPropagation();
    if (!this.menu.open()) this.place();
    this.menu.toggle();
  }

  /** Measured before the panel exists: the trigger is what says where there is room. */
  private place(): void {
    const trigger = this.host.nativeElement.querySelector('.card-menu-trigger');
    const box = trigger?.getBoundingClientRect();
    if (!box) return;

    const margin = NoteCardMenuComponent.MARGIN;
    const below = window.innerHeight - box.bottom - margin;
    const above = box.top - margin;
    const dropUp = above > below;

    this.panelStyle.set({
      right: `${Math.round(window.innerWidth - box.right)}px`,
      [dropUp ? 'bottom' : 'top']:
        `${Math.round(dropUp ? window.innerHeight - box.top + 4 : box.bottom + 4)}px`,
      maxHeight: `${Math.round(Math.max(above, below) - 4)}px`,
    });
  }

  protected move(spaceId: string): void {
    this.moveRequested.emit(spaceId);
    this.menu.close();
  }

  protected file(folderId: string | null): void {
    this.fileRequested.emit(folderId);
    this.menu.close();
  }

  /** The entries that carry nothing: an output, fired and folded away. */
  protected run(action: { emit: (value: void) => void }): void {
    action.emit();
    this.menu.close();
  }

  protected onDeleteClick(): void {
    if (!this.confirmingDelete()) {
      this.confirmingDelete.set(true);
      return;
    }
    this.deleteRequested.emit();
    this.menu.close();
  }
}
