import { ChangeDetectionStrategy, Component, input, output } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';

/**
 * One of the few options. A key, translated in the template: a label translated by the
 * caller is frozen in the language it was built in.
 */
export interface Segment {
  readonly id: string;
  readonly labelKey: string;
}

/**
 * A handful of options, all of them on screen at once.
 *
 * A menu for three choices hides two of them behind a click for nothing, and a native
 * `<select>` brings the operating system's chrome with it — a different border, a different
 * arrow, a different focus ring and, on Windows, a different font. Three radio-like buttons
 * read better and cost a row of nothing.
 *
 * `role="radiogroup"` and not a group of toggles: exactly one is chosen at all times, and
 * that is what tells a screen reader which one.
 */
@Component({
  selector: 'app-segmented-choice',
  imports: [TranslocoPipe],
  templateUrl: './segmented-choice.component.html',
  styleUrl: './segmented-choice.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class SegmentedChoiceComponent {
  readonly label = input.required<string>();
  /** Stable, unlike `label`, which is translated: this is what the tests address. */
  readonly kind = input.required<string>();
  readonly segments = input.required<readonly Segment[]>();
  readonly currentId = input.required<string>();

  readonly chosen = output<string>();

  protected pick(id: string): void {
    if (id === this.currentId()) return;

    this.chosen.emit(id);
  }
}
