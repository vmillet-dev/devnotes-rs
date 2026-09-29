import { Injectable, inject } from '@angular/core';
import { NotesRepository } from '@core/data/notes.repository';
import { NoteDraft } from '@core/model/note.model';
import { ErrorNotifier } from '@core/services/errors/error-notifier.service';
import { StatusNotifier } from '@core/services/notifications/status.service';
import { NotesRevision } from '@core/state/notes-revision';
import { ToolResult } from './tool.model';

/** Where a tool's result goes, as the dialog settled it. */
export interface ToolNotePlacement {
  readonly title: string;
  readonly spaceId: string;
  readonly folderId: string | null;
  readonly tags: readonly string[];
  /** "Outils / Hash / HMAC": the card says where the note came from. */
  readonly source: string;
  /** "Travail › Scripts", for the line that says where it went. */
  readonly place: string;
}

/**
 * A tool's result written as a note like any other: the canvas learns of it through
 * `NotesRevision`, wherever it is, and the status line says where it went.
 */
@Injectable({ providedIn: 'root' })
export class ToolNotes {
  private readonly repository = inject(NotesRepository);
  private readonly notifier = inject(ErrorNotifier);
  private readonly status = inject(StatusNotifier);
  private readonly revision = inject(NotesRevision);

  async save(result: ToolResult, placement: ToolNotePlacement): Promise<boolean> {
    const draft: NoteDraft = {
      spaceId: placement.spaceId,
      folderId: placement.folderId,
      title: placement.title,
      language: result.language,
      content: result.content,
      source: placement.source,
      tags: placement.tags,
      pinned: false,
      lifecycle: { kind: 'permanent' },
      kind: result.kind,
      priority: 'none',
      items: [],
    };

    const created = await this.notifier.attempt('errors.noteCreateFailed', () =>
      this.repository.create(draft),
    );
    if (created === null) return false;

    this.revision.bump();
    this.status.notify({ key: 'tools.saved', params: { title: created.title, place: placement.place } });
    return true;
  }
}
