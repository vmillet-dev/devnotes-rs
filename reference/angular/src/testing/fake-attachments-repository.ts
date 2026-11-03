import { guard } from './fail-next';
import { AttachmentsRepository } from '@core/data/attachments.repository';
import { Attachment } from '@core/model/note.model';

/** `read` returns a stub `data:` URI: what the front owns is *when* it asks for bytes. */
export class FakeAttachmentsRepository implements Pick<AttachmentsRepository, keyof AttachmentsRepository> {
  private attachments: readonly Attachment[];
  private nextId = 0;

  /** When set, the next call to any method rejects with this error, then clears. */
  failNext: Error | null = null;

  /** Reads recorded, so a spec can assert nothing is preloaded in bulk. */
  reads: string[] = [];

  opened: string[] = [];
  savedAs: { id: string; path: string } | null = null;

  constructor(attachments: readonly Attachment[] = []) {
    this.attachments = attachments;
  }

  loadFor(noteId: string): Promise<readonly Attachment[]> {
    return guard(this, () => this.attachments.filter((attachment) => attachment.noteId === noteId));
  }

  attach(noteId: string, path: string): Promise<Attachment> {
    return guard(this, () => {
      const attachment: Attachment = {
        id: `fake-attachment-${++this.nextId}`,
        noteId,
        fileName: path.split(/[/\\]/).pop() ?? path,
        mimeType: 'image/png',
        byteSize: 2048,
        createdAt: new Date(),
      };
      this.attachments = [...this.attachments, attachment];
      return attachment;
    });
  }

  read(id: string): Promise<string> {
    return guard(this, () => {
      this.reads.push(id);
      return `data:image/png;base64,${id}`;
    });
  }

  open(id: string): Promise<void> {
    return guard(this, () => {
      this.opened.push(id);
    });
  }

  saveAs(id: string, path: string): Promise<void> {
    return guard(this, () => {
      this.savedAs = { id, path };
    });
  }

  attachClipboardImage(noteId: string, fileName: string): Promise<Attachment> {
    return guard(this, () => {
      const attachment: Attachment = {
        id: `fake-attachment-${++this.nextId}`,
        noteId,
        fileName: `${fileName}.png`,
        mimeType: 'image/png',
        byteSize: 4096,
        createdAt: new Date(),
      };
      this.attachments = [...this.attachments, attachment];
      return attachment;
    });
  }

  delete(id: string): Promise<void> {
    return guard(this, () => {
      this.attachments = this.attachments.filter((attachment) => attachment.id !== id);
    });
  }
}
