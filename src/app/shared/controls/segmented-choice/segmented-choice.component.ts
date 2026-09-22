import { ChangeDetectionStrategy, Component, input, output } from '@angular/core';

/** One of the few options, already translated by whoever knows what they mean. */
export interface Segment {
  readonly id: string;
  readonly label: string;
}

/**
 * A handful of options, all of them on screen at once.
 *
 * ⚠️ A menu for three choices hides two of them behind a click for nothing, and a native
 * `<select>` brings the operating system's chrome with it — a different border, a different
 * arrow, a different focus ring and, on Windows, a different font. Three radio-like buttons
 * read better and cost a row of nothing.
 *
 * ⚠️ `role="radiogroup"` and not a group of toggles: exactly one is chosen at all times, and
 * that is what tells a screen reader which one.
 */
@Component({
  selector: 'app-segmented-choice',
  templateUrl: './segmented-choice.component.html',
  styleUrl: './segmented-choice.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class SegmentedChoiceComponent {
  readonly label = input.required<string>();
  readonly segments = input.required<readonly Segment[]>();
  readonly currentId = input.required<string>();

  readonly chosen = output<string>();

  protected pick(id: string): void {
    if (id === this.currentId()) return;

    this.chosen.emit(id);
  }
}
