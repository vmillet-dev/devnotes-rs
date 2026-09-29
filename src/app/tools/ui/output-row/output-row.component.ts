import { ChangeDetectionStrategy, Component, input } from '@angular/core';
import { CopyValueComponent } from '../copy-value/copy-value.component';

/** One result of a tool: its name, what it measures, the value itself and its copy button. */
@Component({
  selector: 'app-output-row',
  imports: [CopyValueComponent],
  templateUrl: './output-row.component.html',
  styleUrl: './output-row.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
  host: { '[class.matched]': 'matched()' },
})
export class OutputRowComponent {
  readonly name = input.required<string>();
  readonly value = input.required<string>();
  /** "256 bits": what the value is, beside its name. */
  readonly meta = input('');
  /** Outlined, when this is the one that answers what was asked: a signature it matches. */
  readonly matched = input(false);
}
