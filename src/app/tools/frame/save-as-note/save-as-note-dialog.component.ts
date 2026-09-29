import {
  ChangeDetectionStrategy,
  Component,
  ElementRef,
  afterNextRender,
  computed,
  inject,
  input,
  linkedSignal,
  output,
  signal,
  viewChild,
} from '@angular/core';
import { TranslocoPipe, TranslocoService } from '@jsverse/transloco';
import { LANGUAGE_LABELS } from '@core/model/language.model';
import { ToolNotes } from '@core/services/tools/tool-notes';
import { ToolResult } from '@core/services/tools/tool.model';
import { FoldersStore } from '@core/state/folders.store';
import { SpacesStore } from '@core/state/spaces.store';
import { ChoiceMenuComponent, ChoiceOption } from '@shared/controls/choice-menu/choice-menu.component';
import { DialogComponent } from '@shared/layout/dialog/dialog.component';

/** Commas only: a tag may hold a space, and Rust normalises what it keeps. */
function tagsOf(text: string): string[] {
  return text
    .split(',')
    .map((tag) => tag.trim())
    .filter((tag) => tag !== '');
}

/**
 * One dialog for every tool. It asks where the note goes and what it is called; what it holds,
 * and in which language, the tool has already decided.
 */
@Component({
  selector: 'app-save-as-note-dialog',
  imports: [ChoiceMenuComponent, DialogComponent, TranslocoPipe],
  templateUrl: './save-as-note-dialog.component.html',
  styleUrl: './save-as-note-dialog.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class SaveAsNoteDialogComponent {
  private readonly spacesStore = inject(SpacesStore);
  private readonly foldersStore = inject(FoldersStore);
  private readonly notes = inject(ToolNotes);
  private readonly transloco = inject(TranslocoService);

  readonly result = input.required<ToolResult>();
  readonly toolId = input.required<string>();

  readonly closed = output<void>();

  private readonly titleField = viewChild.required<ElementRef<HTMLInputElement>>('titleField');

  protected readonly title = linkedSignal(() => {
    const { key, params } = this.result().title;
    return this.transloco.translate(key, params);
  });

  protected readonly spaceId = signal<string | null>(
    this.spacesStore.activeSpaceId() ?? this.spacesStore.spaces()[0]?.id ?? null,
  );

  /** The folder open in the notes, when it is in the space chosen; none once another is. */
  protected readonly folderId = linkedSignal<string | null, string | null>({
    source: () => this.spaceId(),
    computation: (spaceId, previous) => {
      const active = this.foldersStore.activeFolder();
      return previous === undefined && active?.spaceId === spaceId ? active.id : null;
    },
  });

  protected readonly tags = signal('');
  protected readonly saving = signal(false);

  protected readonly spaces = computed<readonly ChoiceOption[]>(() =>
    this.spacesStore.spaces().map((space) => ({ id: space.id, name: space.name })),
  );

  protected readonly folders = computed<readonly ChoiceOption[]>(() => {
    const spaceId = this.spaceId();
    return spaceId === null
      ? []
      : this.foldersStore
          .foldersOf(spaceId)
          .map((folder) => ({ id: folder.id, name: folder.name, colour: folder.colour }));
  });

  protected readonly languageLabel = computed(() => LANGUAGE_LABELS[this.result().language]);

  protected readonly canSave = computed(
    () => this.title().trim() !== '' && this.spaceId() !== null && !this.saving(),
  );

  constructor() {
    afterNextRender(() => this.titleField().nativeElement.select());
  }

  protected onTitle(event: Event): void {
    this.title.set((event.target as HTMLInputElement).value);
  }

  protected onTags(event: Event): void {
    this.tags.set((event.target as HTMLInputElement).value);
  }

  protected onSpace(id: string | null): void {
    this.spaceId.set(id);
  }

  protected onFolder(id: string | null): void {
    this.folderId.set(id);
  }

  protected async save(event: Event): Promise<void> {
    event.preventDefault();
    const spaceId = this.spaceId();
    if (!this.canSave() || spaceId === null) return;

    this.saving.set(true);
    const saved = await this.notes.save(this.result(), {
      title: this.title().trim(),
      spaceId,
      folderId: this.folderId(),
      tags: tagsOf(this.tags()),
      source: [
        this.transloco.translate('tools.title'),
        this.transloco.translate(`tools.${this.toolId()}.name`),
      ].join(' / '),
      place: this.place(spaceId),
    });
    this.saving.set(false);

    if (saved) {
      this.closed.emit();
    }
  }

  private place(spaceId: string): string {
    const space = this.spacesStore.spaces().find((candidate) => candidate.id === spaceId)?.name ?? '';
    const folder = this.folders().find((candidate) => candidate.id === this.folderId())?.name;
    return folder ? `${space} › ${folder}` : space;
  }
}
