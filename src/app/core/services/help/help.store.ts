import { Injectable, signal } from '@angular/core';
import { GUIDE_CHAPTERS, GuideChapter } from './guide.model';

/**
 * Which chapter of the guide is open, if any.
 *
 * ⚠️ A store rather than a boolean inside the About menu, because the guide is reached from
 * **the thing it explains** as well as from the menu: an empty canvas offers the chapter
 * about notes, an empty board the one about folders. Neither is anywhere near the titlebar.
 */
@Injectable({ providedIn: 'root' })
export class HelpStore {
  private readonly _chapter = signal<GuideChapter | null>(null);

  /** `null` when the guide is closed. */
  readonly chapter = this._chapter.asReadonly();

  open(chapter: GuideChapter = GUIDE_CHAPTERS[0]): void {
    this._chapter.set(chapter);
  }

  close(): void {
    this._chapter.set(null);
  }
}
