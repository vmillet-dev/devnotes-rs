import { ChangeDetectionStrategy, Component, computed, inject, input, output } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { FormPartRow } from '@core/model/http.model';
import { FileDialogService } from '@core/services/dialogs/file-dialog.service';

type Field = 'key' | 'value' | 'description';

const BLANK: FormPartRow = { enabled: true, key: '', value: '', description: '', file: false };

/** A multipart body's fields: a text each, or a file by its path, read when the request is sent. */
@Component({
  selector: 'app-form-parts-table',
  imports: [TranslocoPipe],
  templateUrl: './form-parts-table.component.html',
  styleUrl: './form-parts-table.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class FormPartsTableComponent {
  readonly parts = input.required<readonly FormPartRow[]>();
  readonly partsChange = output<readonly FormPartRow[]>();

  private readonly files = inject(FileDialogService);

  protected readonly shown = computed(() => [...this.parts(), BLANK]);

  protected isBlank(index: number): boolean {
    return index === this.parts().length;
  }

  protected onInput(index: number, field: Field, event: Event): void {
    this.replace(index, { [field]: (event.target as HTMLInputElement).value });
  }

  protected onToggle(index: number, event: Event): void {
    this.replace(index, { enabled: (event.target as HTMLInputElement).checked });
  }

  /** A text becomes a file and back; what was typed is no path, so it is let go. */
  protected switchKind(index: number): void {
    const part = this.parts()[index] ?? BLANK;
    this.replace(index, { file: !part.file, value: '' });
  }

  protected async choose(index: number): Promise<void> {
    const path = await this.files.pickFile();
    if (path !== null) this.replace(index, { value: path, file: true });
  }

  protected remove(index: number): void {
    this.partsChange.emit(this.parts().filter((_, at) => at !== index));
  }

  private replace(index: number, change: Partial<FormPartRow>): void {
    const parts = [...this.parts()];
    parts[index] = { ...(parts[index] ?? BLANK), ...change };
    this.partsChange.emit(parts);
  }
}
