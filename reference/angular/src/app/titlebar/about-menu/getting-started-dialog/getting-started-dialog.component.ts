import { ChangeDetectionStrategy, Component, computed, inject, input, output, signal } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { GUIDE_CHAPTERS, GuideChapter } from '@core/services/help/guide.model';
import { SettingsStore } from '@core/services/settings/settings.store';
import { DEFAULT_SHORTCUTS } from '@core/services/shortcuts/shortcut.model';
import { CHECK_KEY } from '@notes/canvas-keys';
import { DialogComponent } from '@shared/layout/dialog/dialog.component';
import { GuideFigureComponent } from './guide-figure/guide-figure.component';

/**
 * The written guide, **one chapter at a time**.
 *
 * It was ten chapters of prose in one scrolling panel — roughly 3 400 characters, all of
 * it true and none of it looked at, which is the worst return a help surface can have. A
 * chapter is now a schematic of the screen it is about and two sentences beside it: short
 * enough to be read standing up, and ten of them is a walk rather than a wall.
 *
 * It opens **at a chapter**, because the canvas and the board link straight into the one
 * that explains them. That is what `HelpStore` exists for.
 */
@Component({
  selector: 'app-getting-started-dialog',
  imports: [DialogComponent, TranslocoPipe, GuideFigureComponent],
  templateUrl: './getting-started-dialog.component.html',
  styleUrl: './getting-started-dialog.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class GettingStartedDialogComponent {
  private readonly settings = inject(SettingsStore);

  readonly startAt = input<GuideChapter>(GUIDE_CHAPTERS[0]);

  readonly closed = output<void>();

  protected readonly chapters = GUIDE_CHAPTERS;

  private readonly index = signal<number | null>(null);

  /** The chapter asked for until the reader walks away from it. */
  protected readonly at = computed(() => this.index() ?? this.chapters.indexOf(this.startAt()));

  protected readonly chapter = computed<GuideChapter>(() => this.chapters[this.at()] ?? this.chapters[0]);

  protected readonly isFirst = computed(() => this.at() === 0);
  protected readonly isLast = computed(() => this.at() === this.chapters.length - 1);

  /**
   * Interpolated into every chapter, because Transloco replaces an unknown `{{name}}`
   * with the empty string — a body cannot spell a key out. The quick-paste one follows
   * the preference rather than the combination that shipped.
   */
  protected readonly keys = computed(() => ({
    palette: this.settings.paletteShortcut(),
    capture: DEFAULT_SHORTCUTS.capture,
    newNote: DEFAULT_SHORTCUTS.newNote,
    check: CHECK_KEY,
  }));

  protected go(by: number): void {
    const next = Math.min(this.chapters.length - 1, Math.max(0, this.at() + by));
    this.index.set(next);
  }

  protected jumpTo(index: number): void {
    this.index.set(index);
  }
}
