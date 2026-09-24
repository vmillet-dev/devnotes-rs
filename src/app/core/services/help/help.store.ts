import { Injectable, computed, signal } from '@angular/core';
import { GUIDE_CHAPTERS, GuideChapter } from './guide.model';

/** One value rather than four booleans, which would allow a state with two stacked. */
export type HelpPanel = 'whatsNew' | 'gettingStarted' | 'shortcuts' | 'about';

/**
 * Which help panel is up, and which chapter of the guide.
 *
 * A store rather than a signal inside the About menu: the guide is reached from **the thing
 * it explains** too — an empty canvas, an empty board — and the editor puts every panel away
 * when it opens, since they are drawn over it.
 */
@Injectable({ providedIn: 'root' })
export class HelpStore {
  private readonly _chapter = signal<GuideChapter | null>(null);
  private readonly _panel = signal<HelpPanel | null>(null);

  /** `null` when the guide is closed. */
  readonly chapter = this._chapter.asReadonly();

  /** `null` when nothing is up. The guide is a panel like the others. */
  readonly panel = computed<HelpPanel | null>(() =>
    this._chapter() !== null ? 'gettingStarted' : this._panel(),
  );

  open(chapter: GuideChapter = GUIDE_CHAPTERS[0]): void {
    this._panel.set(null);
    this._chapter.set(chapter);
  }

  show(panel: Exclude<HelpPanel, 'gettingStarted'>): void {
    this._chapter.set(null);
    this._panel.set(panel);
  }

  close(): void {
    this._chapter.set(null);
    this._panel.set(null);
  }
}
