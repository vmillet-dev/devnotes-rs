import { ChangeDetectionStrategy, Component, input } from '@angular/core';
import { CopyValueComponent } from '../copy-value/copy-value.component';

/** A row of a results card: the name, the value, and a copy icon at the end. */
@Component({
  selector: 'app-result-row',
  imports: [CopyValueComponent],
  templateUrl: './result-row.component.html',
  styleUrl: './result-row.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ResultRowComponent {
  readonly name = input.required<string>();
  readonly value = input.required<string>();
  /** What the icon copies when it is not what is shown: the exact value behind a rounded one. */
  readonly copied = input<string | null>(null);
  /** A reading, not a value: nothing to paste anywhere. */
  readonly copyable = input(true);
}
