import { ChangeDetectionStrategy, Component, computed, inject, input, output, signal } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { StatusNotifier } from '@core/services/notifications/status.service';
import { TOOL_CATALOGUE } from '@core/services/tools/tool.model';
import { ToolsStore } from '@core/services/tools/tools.store';
import { IconComponent } from '@shared/icon/icon.component';
import { NotePickerComponent, PickedNote } from './note-picker/note-picker.component';

/**
 * « Importer depuis une note » beside an input the open tool lists in the catalogue's
 * `fromNote`, and nothing beside one it does not. The note is copied in once: nothing ties the
 * tool to it afterwards.
 */
@Component({
  selector: 'app-note-import',
  imports: [IconComponent, NotePickerComponent, TranslocoPipe],
  templateUrl: './note-import.component.html',
  styleUrl: './note-import.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class NoteImportComponent {
  private readonly catalogue = inject(TOOL_CATALOGUE, { optional: true }) ?? [];
  private readonly store = inject(ToolsStore);
  private readonly status = inject(StatusNotifier);

  /** The input's name in the tool's `fromNote`. */
  readonly field = input.required<string>();
  /** For a tool with two inputs alike: « Importer A depuis une note ». */
  readonly label = input<string | null>(null);
  readonly testId = input<string | null>(null);

  readonly imported = output<PickedNote>();

  protected readonly accepts = computed(() => {
    const id = this.store.openId();
    return this.catalogue.find((tool) => tool.id === id)?.fromNote?.[this.field()] ?? null;
  });

  protected readonly picking = signal(false);

  protected onPicked(note: PickedNote): void {
    this.picking.set(false);
    this.imported.emit(note);
    this.status.notify(
      note.title
        ? { key: 'tools.fromNote.done', params: { title: note.title } }
        : { key: 'tools.fromNote.doneUntitled' },
    );
  }
}
