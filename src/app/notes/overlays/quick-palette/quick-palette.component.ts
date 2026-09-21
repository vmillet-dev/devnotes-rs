import { ChangeDetectionStrategy, Component, input, output } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { DialogComponent } from '@shared/layout/dialog/dialog.component';
import { LanguageBadgeComponent } from '@notes/ui/language-badge/language-badge.component';
import { Note } from '@core/model/note.model';

const SNIPPET_LINES = 2;

/**
 * ⚠️ `aria-activedescendant` and not a moved focus, which would lose what is being typed
 * — and the rows are out of the tab order so that it stays true. Tab was released to the
 * DOM when Enter took over opening, and it walked a list of stops nothing drew a ring on
 * while the highlight stayed where the arrows had left it: two notions of "the current
 * row", one of them invisible (#283). Tab is a slower arrow now, and the copy control is
 * reached with `Ctrl+C` rather than by walking to it.
 */
@Component({
  selector: 'app-quick-palette',
  imports: [DialogComponent, LanguageBadgeComponent, TranslocoPipe],
  templateUrl: './quick-palette.component.html',
  styleUrl: './quick-palette.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class QuickPaletteComponent {
  readonly query = input('');
  readonly results = input.required<readonly Note[]>();
  readonly highlighted = input(0);
  readonly canCreate = input(false);

  readonly queryChanged = output<string>();
  readonly highlightMoved = output<number>();
  readonly highlightSet = output<number>();
  readonly chosen = output<void>();
  /** The note itself: the canvas filters may hide it, and then nobody can look it up. */
  readonly openRequested = output<Note>();
  readonly closed = output<void>();

  protected snippetOf(note: Note): string {
    return note.content.split('\n').slice(0, SNIPPET_LINES).join('\n');
  }

  protected optionId(index: number): string {
    return `palette-option-${index}`;
  }

  /** `null` on the create row, which sits past the results and is not a note to open. */
  private highlightedNote(): Note | null {
    return this.results()[this.highlighted()] ?? null;
  }

  /** ⚠️ A field with a selection in it: `Ctrl+C` there means the selected text. */
  private hasSelectedText(event: KeyboardEvent): boolean {
    const field = event.target;
    return field instanceof HTMLInputElement && field.selectionStart !== field.selectionEnd;
  }

  protected onKeydown(event: KeyboardEvent): void {
    // ⚠️ `Ctrl+C` and not a bare `c`, which the field would simply type. It is the letter
    // the canvas already copies with, and the only modifier a text field leaves free.
    if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'c') {
      if (this.hasSelectedText(event)) return;

      event.preventDefault();
      this.chosen.emit();
      return;
    }

    switch (event.key) {
      case 'ArrowDown':
        event.preventDefault();
        this.highlightMoved.emit(1);
        break;
      case 'ArrowUp':
        event.preventDefault();
        this.highlightMoved.emit(-1);
        break;
      // The focus never leaves the field, so Tab has nowhere useful to go; taken here it
      // is the same move as the arrows, which is what a hand already on Tab expects.
      case 'Tab':
        event.preventDefault();
        this.highlightMoved.emit(event.shiftKey ? -1 : 1);
        break;
      // Opens, as a click on the row does and as Enter does everywhere else in the
      // application. The create row has nothing to open, so it is still `chosen`.
      case 'Enter': {
        event.preventDefault();
        const note = this.highlightedNote();
        if (note) {
          this.openRequested.emit(note);
        } else {
          this.chosen.emit();
        }
        break;
      }
    }
  }
}
