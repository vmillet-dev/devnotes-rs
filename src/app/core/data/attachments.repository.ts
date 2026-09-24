import { Injectable } from '@angular/core';
import { commands } from '@core/ipc/bindings';
import { unwrap } from '@core/ipc/ipc.error';
import { Attachment } from '@core/model/note.model';
import { toAttachment } from './note.mapper';

/**
 * ⚠️ `read` answers a `data:` URI, the one form an `<img>` accepts under the WebView's
 * CSP. It weighs a third more than the file, so the bytes are only read on demand.
 */
@Injectable({ providedIn: 'root' })
export class AttachmentsRepository {
  async loadFor(noteId: string): Promise<readonly Attachment[]> {
    return unwrap('list_attachments', await commands.listAttachments(noteId)).map(toAttachment);
  }

  /** `path` comes from the native file picker; the copy happens on the Rust side. */
  async attach(noteId: string, path: string): Promise<Attachment> {
    return toAttachment(unwrap('attach_file', await commands.attachFile(noteId, path)));
  }

  async read(id: string): Promise<string> {
    return unwrap('read_attachment', await commands.readAttachment(id));
  }

  async open(id: string): Promise<void> {
    unwrap('open_attachment', await commands.openAttachment(id));
  }

  async saveAs(id: string, path: string): Promise<void> {
    unwrap('save_attachment', await commands.saveAttachment(id, path));
  }

  /** The bytes never cross the bridge: the native side re-reads the clipboard itself. */
  async attachClipboardImage(noteId: string, fileName: string): Promise<Attachment> {
    return toAttachment(
      unwrap('attach_clipboard_image', await commands.attachClipboardImage(noteId, fileName)),
    );
  }

  async delete(id: string): Promise<void> {
    unwrap('delete_attachment', await commands.deleteAttachment(id));
  }
}
