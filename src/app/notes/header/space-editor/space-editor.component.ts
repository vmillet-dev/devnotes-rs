import {
  ChangeDetectionStrategy,
  Component,
  ElementRef,
  computed,
  inject,
  input,
  linkedSignal,
  output,
  resource,
  signal,
  viewChild,
} from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { Space } from '@core/model/space.model';
import { TranslationRef } from '@core/services/i18n/translation-ref.model';
import { SpacesStore } from '@core/state/spaces.store';
import { ChoiceMenuComponent, ChoiceOption } from '@shared/controls/choice-menu/choice-menu.component';

/**
 * Pin, rename, delete — the three things a space can be told to do, in one panel so the
 * switcher and the library rail cannot drift apart. `folder-editor` is its twin: it acts
 * itself, and its host only hears `finished`.
 */
@Component({
  selector: 'app-space-editor',
  imports: [TranslocoPipe, ChoiceMenuComponent],
  templateUrl: './space-editor.component.html',
  styleUrl: './space-editor.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class SpaceEditorComponent {
  readonly space = input.required<Space>();
  /** A space cannot be its own refuge: the cascade would take the notes after the move. */
  readonly moveTargets = input.required<readonly Space[]>();

  /** After a rename or a deletion; pinning leaves the panel where it is. */
  readonly finished = output<void>();

  private readonly spaces = inject(SpacesStore);

  protected readonly targetChoices = computed<readonly ChoiceOption[]>(() =>
    this.moveTargets().map((target) => ({ id: target.id, name: target.name })),
  );

  /**
   * The refuge is state now, where the `<select>` held it: a menu writes what it was
   * asked for and has nothing to read back. Keyed on the list, so a space appearing or
   * disappearing lands on the first one rather than on an id that no longer exists.
   */
  protected readonly refuge = linkedSignal<readonly ChoiceOption[], string | null>({
    source: this.targetChoices,
    computation: (targets, previous) => {
      const kept = targets.find((target) => target.id === previous?.value);
      return (kept ?? targets[0])?.id ?? null;
    },
  });

  /** Two steps: the WebView blocks on a native `confirm()`. */
  protected readonly confirmingDelete = signal(false);

  /** Counted by Rust when the panel opens, so the sentence is there before the first click. */
  private readonly contents = resource({
    params: () => this.space().id,
    loader: ({ params }) => this.spaces.contentsOf(params),
  });

  /** What deleting takes or moves, in words: the notes and where they go, then the folders. */
  protected readonly consequences = computed<readonly TranslationRef[]>(() => {
    const contents = this.contents.hasValue() ? this.contents.value() : null;
    const refuge = this.targetChoices().find((target) => target.id === this.refuge());
    if (!contents || !refuge) return [];

    const notes: TranslationRef =
      contents.notes === 0
        ? { key: 'notes.deleteSpaceNoNotes' }
        : { key: 'notes.deleteSpaceNotes', params: { count: contents.notes, refuge: refuge.name } };
    return contents.folders === 0
      ? [notes]
      : [notes, { key: 'notes.deleteSpaceFolders', params: { count: contents.folders } }];
  });

  private readonly renameInput = viewChild<ElementRef<HTMLInputElement>>('renameInput');

  focusName(): void {
    this.renameInput()?.nativeElement.focus();
  }

  protected submitRename(event: Event, name: string): void {
    event.preventDefault();
    if (!name.trim()) return;

    void this.spaces.renameSpace(this.space().id, name);
    this.finished.emit();
  }

  protected togglePinned(): void {
    void this.spaces.togglePinned(this.space().id);
  }

  protected onDeleteClick(targetSpaceId: string): void {
    if (!targetSpaceId) return;

    if (!this.confirmingDelete()) {
      this.confirmingDelete.set(true);
      return;
    }
    void this.spaces.deleteSpace(this.space().id, targetSpaceId);
    this.finished.emit();
  }
}
