import { Injectable } from '@angular/core';
import { commands } from '@core/ipc/bindings';
import type { LibraryEntry, Registry } from '@core/ipc/bindings';
import { unwrap } from '@core/ipc/ipc.error';

export type { LibraryEntry, Registry };

/**
 * `Registry` crosses as itself: there is no wire shape to put back, so there is no
 * mapper. `createdAt` is never read as a date — the list is ordered as Rust gives it.
 */
@Injectable({ providedIn: 'root' })
export class LibrariesRepository {
  async list(): Promise<Registry> {
    return unwrap('list_libraries', await commands.listLibraries());
  }

  /** Adds one and leaves it closed: opening it is a second, deliberate gesture. */
  async create(name: string): Promise<LibraryEntry> {
    return unwrap('create_library', await commands.createLibrary(name));
  }

  /**
   * The library is **closed** by the time this returns, so every command answers
   * `Locked` afterwards. The caller's next move is to send the shell back to the gate:
   * the other library has its own passphrase, and asking for it is the only proof the
   * right one is open.
   */
  async open(id: string): Promise<void> {
    unwrap('open_library', await commands.openLibrary(id));
  }

  async rename(id: string, name: string): Promise<void> {
    unwrap('rename_library', await commands.renameLibrary(id, name));
  }

  /** Irreversible, and it takes the notes, the attachments and the copies with it. */
  async delete(id: string): Promise<void> {
    unwrap('delete_library', await commands.deleteLibrary(id));
  }
}
