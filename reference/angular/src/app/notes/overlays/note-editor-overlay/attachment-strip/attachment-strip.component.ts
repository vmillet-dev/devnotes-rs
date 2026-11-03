import { ChangeDetectionStrategy, Component, computed, input, output, signal } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { ATTACHMENT_MAX_MEGABYTES, Attachment } from '@core/model/note.model';

const BYTES_PER_KB = 1024;

/** One preview at a time: a `data:` URI weighs a third more than the file. */
@Component({
  selector: 'app-attachment-strip',
  imports: [TranslocoPipe],
  templateUrl: './attachment-strip.component.html',
  styleUrl: './attachment-strip.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class AttachmentStripComponent {
  readonly attachments = input.required<readonly Attachment[]>();
  readonly isBusy = input(false);
  readonly previewId = input<string | null>(null);
  readonly previewData = input<string | null>(null);

  readonly addRequested = output<void>();
  readonly openRequested = output<string>();
  readonly saveRequested = output<string>();
  readonly removeRequested = output<string>();
  readonly previewToggled = output<string>();
  readonly zoomRequested = output<void>();

  protected readonly confirmingRemove = signal<string | null>(null);
  protected readonly maxMegabytes = ATTACHMENT_MAX_MEGABYTES;

  protected readonly previewed = computed<Attachment | null>(() => {
    const id = this.previewId();
    return this.attachments().find((attachment) => attachment.id === id) ?? null;
  });

  protected isImage(attachment: Attachment): boolean {
    return attachment.mimeType.startsWith('image/');
  }

  /** Rounded up: "0 kB" for a non-empty file would be wrong. */
  protected sizeInKb(attachment: Attachment): number {
    return Math.max(1, Math.ceil(attachment.byteSize / BYTES_PER_KB));
  }

  protected onRemoveClick(id: string): void {
    if (this.confirmingRemove() !== id) {
      this.confirmingRemove.set(id);
      return;
    }
    this.confirmingRemove.set(null);
    this.removeRequested.emit(id);
  }
}
