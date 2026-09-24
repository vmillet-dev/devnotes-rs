import { ChangeDetectionStrategy, Component, input, output } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { Placeholder } from '@core/model/note.model';

export interface PlaceholderValue {
  readonly name: string;
  readonly value: string;
}

/**
 * Holds no state: the parent carries the values, because one stores them and the other
 * throws them away. An empty field shows its default as a suggestion and never copies
 * it — the day the snippet's default changes, a stored copy would win.
 */
@Component({
  selector: 'app-placeholder-fields',
  imports: [TranslocoPipe],
  templateUrl: './placeholder-fields.component.html',
  styleUrl: './placeholder-fields.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class PlaceholderFieldsComponent {
  readonly placeholders = input.required<readonly Placeholder[]>();

  /** The current values, by field name. Absent means nothing was typed. */
  readonly values = input.required<Record<string, string>>();

  readonly changed = output<PlaceholderValue>();

  protected valueOf(name: string): string {
    return this.values()[name] ?? '';
  }
}
