import { Injectable, inject, signal } from '@angular/core';
import { NotesRepository } from '@core/data/notes.repository';
import type { DrawingRef } from '@core/model/note.model';
import { NotesRevision } from './notes-revision';

interface Kept<Stamp> {
  readonly stamp: Stamp;
  readonly uri: Promise<string | null>;
}

/**
 * A Dessin's pictures, asked for where they are shown and kept: a list carries none, so that a
 * search opens no picture. A card's is kept until the drawing is saved again, a Note's embed
 * until any note is.
 */
@Injectable({ providedIn: 'root' })
export class DrawingPicturesStore {
  private readonly repository = inject(NotesRepository);
  private readonly revision = inject(NotesRevision);
  private readonly thumbnails = new Map<string, Kept<string>>();
  private readonly pictures = new Map<string, Kept<number>>();
  private readonly _redrawn = signal<ReadonlyMap<string, number>>(new Map());

  /** Part of a card's stamp: pictures written beside a body left as it was move no date. */
  redrawn(id: string): number {
    return this._redrawn().get(id) ?? 0;
  }

  pictured(id: string): void {
    this._redrawn.update((counts) => new Map(counts).set(id, (counts.get(id) ?? 0) + 1));
  }

  /** An imported drawing has none until it is opened. One that cannot be read is not retaken. */
  async hasPictures(id: string): Promise<boolean> {
    try {
      return (await this.repository.drawingThumbnail(id)) !== null;
    } catch {
      return true;
    }
  }

  /** `stamp` is the drawing's last save: another one makes what is kept stale. */
  thumbnail(id: string, stamp: string): Promise<string | null> {
    return this.kept(this.thumbnails, id, stamp, () => this.repository.drawingThumbnail(id));
  }

  picture(id: string): Promise<string | null> {
    return this.kept(this.pictures, id, this.revision.current(), () => this.repository.drawingPicture(id));
  }

  /** The drawings a Note can embed; none when they cannot be had, which hides the menu. */
  async list(): Promise<readonly DrawingRef[]> {
    try {
      return await this.repository.listDrawings();
    } catch {
      return [];
    }
  }

  /** A picture that cannot be had is none: the card and the embed say so, nothing fails. */
  private kept<Stamp>(
    store: Map<string, Kept<Stamp>>,
    id: string,
    stamp: Stamp,
    ask: () => Promise<string | null>,
  ): Promise<string | null> {
    const known = store.get(id);
    if (known?.stamp === stamp) return known.uri;

    const uri = ask().catch(() => null);
    store.set(id, { stamp, uri });
    return uri;
  }
}
