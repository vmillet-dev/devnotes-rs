import { Injectable, inject, signal } from '@angular/core';
import { NotesRepository } from '@core/data/notes.repository';
import { FieldSpan } from '@core/model/note.model';
import { debounced } from '@core/services/time/debounce';

/** Past this, the code field marks nothing: the incremental search waits from the same length. */
export const FIELD_MARKS_UP_TO = 200_000;

const ASK_AFTER_MS = 250;

/** The spans Rust found, and the text they were found in: they place nothing on another. */
export interface FieldMarks {
  readonly text: string;
  readonly spans: readonly FieldSpan[];
}

/**
 * Where the open snippet's `{{fields}}` lie, asked of Rust as the body is typed, for the code
 * field to mark them. What a field is stays `notes::placeholder`'s.
 */
@Injectable({ providedIn: 'root' })
export class FieldMarksStore {
  private readonly repository = inject(NotesRepository);

  private readonly _marks = signal<FieldMarks | null>(null);
  readonly marks = this._marks.asReadonly();

  private latest: string | null = null;
  private readonly askLater = debounced<string>((text) => void this.ask(text), ASK_AFTER_MS);

  /** `null` when there is nothing to mark: not a snippet, or a text past the limit. */
  follow(text: string | null): void {
    if (text === null || text.length > FIELD_MARKS_UP_TO) {
      this.askLater.cancel();
      this.latest = null;
      this._marks.set(null);
      return;
    }
    this.latest = text;
    this.askLater(text);
  }

  /** A highlight, not a write: an answer that fails leaves the code unmarked and says nothing. */
  private async ask(text: string): Promise<void> {
    const spans = await this.repository.fieldSpans(text).catch(() => []);
    if (this.latest === text) this._marks.set({ text, spans });
  }
}
