import { ChangeDetectionStrategy, Component, inject, input, output } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import {
  BODY_KINDS,
  BodyAnswer,
  BodyKind,
  FormPartRow,
  KeyValueRow,
  RequestBodyDraft,
} from '@core/model/http.model';
import { FileDialogService } from '@core/services/dialogs/file-dialog.service';
import { FormatterService } from '@core/services/format/formatter.service';
import {
  Segment,
  SegmentedChoiceComponent,
} from '@shared/controls/segmented-choice/segmented-choice.component';
import { FormPartsTableComponent } from '../form-parts-table/form-parts-table.component';
import { KeyValueTableComponent } from '@http/ui/key-value-table/key-value-table.component';

/**
 * A request's body. A JSON one is checked as it is typed and formatted by Prettier; a file is a
 * path, read by Rust when the request is sent.
 */
@Component({
  selector: 'app-body-editor',
  imports: [FormPartsTableComponent, KeyValueTableComponent, SegmentedChoiceComponent, TranslocoPipe],
  templateUrl: './body-editor.component.html',
  styleUrl: './body-editor.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class BodyEditorComponent {
  readonly body = input.required<RequestBodyDraft>();
  /** What Rust says of it: the `Content-Type` it implies, and why a JSON one does not read. */
  readonly answer = input<BodyAnswer | null>(null);

  readonly bodyChange = output<RequestBodyDraft>();

  private readonly formatter = inject(FormatterService);
  private readonly files = inject(FileDialogService);

  protected readonly kinds: readonly Segment[] = BODY_KINDS.map((id) => ({
    id,
    labelKey: `http.body.kinds.${id}`,
  }));

  /** A text survives a switch between JSON and plain text; anything else starts empty. */
  protected onKind(id: string): void {
    const kind = id as BodyKind;
    const body = this.body();
    if (kind === body.kind) return;
    const text = body.kind === 'json' || body.kind === 'text' ? body.text : '';
    switch (kind) {
      case 'json':
      case 'text':
        this.bodyChange.emit({ kind, text });
        return;
      case 'form':
        this.bodyChange.emit({ kind, fields: [] });
        return;
      case 'multipart':
        this.bodyChange.emit({ kind, parts: [] });
        return;
      case 'binary':
        this.bodyChange.emit({ kind, path: '' });
        return;
      case 'none':
        this.bodyChange.emit({ kind });
    }
  }

  protected onText(event: Event): void {
    const body = this.body();
    if (body.kind === 'json' || body.kind === 'text') {
      this.bodyChange.emit({ kind: body.kind, text: (event.target as HTMLTextAreaElement).value });
    }
  }

  protected onFields(fields: readonly KeyValueRow[]): void {
    this.bodyChange.emit({ kind: 'form', fields });
  }

  protected onParts(parts: readonly FormPartRow[]): void {
    this.bodyChange.emit({ kind: 'multipart', parts });
  }

  protected async format(): Promise<void> {
    const body = this.body();
    if (body.kind !== 'json') return;
    const answer = await this.formatter.format(body.text, 'json', 0, '  ');
    if (answer.kind === 'formatted') this.bodyChange.emit({ kind: 'json', text: answer.text });
  }

  protected async chooseFile(): Promise<void> {
    const path = await this.files.pickFile();
    if (path !== null) this.bodyChange.emit({ kind: 'binary', path });
  }
}
