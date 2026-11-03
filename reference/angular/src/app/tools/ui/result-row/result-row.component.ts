import { ChangeDetectionStrategy, Component, computed, input } from '@angular/core';
import { CopyValueComponent } from '../copy-value/copy-value.component';

/**
 * Past this many characters a value no longer fits beside its name: it goes under it. Above a
 * SHA-512 in hexadecimal, which still reads in a column with the digests around it.
 */
export const LONG_RESULT = 200;

/**
 * A row of a results card: the name, the value, and a copy icon at the end. A long value takes
 * the width of the card under its name, held to a height it scrolls in.
 */
@Component({
  selector: 'app-result-row',
  imports: [CopyValueComponent],
  templateUrl: './result-row.component.html',
  styleUrl: './result-row.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
  host: { '[class.matched]': 'matched()', '[class.long]': 'long()' },
})
export class ResultRowComponent {
  readonly name = input.required<string>();
  readonly value = input.required<string>();
  /** What the icon copies when it is not what is shown: the exact value behind a rounded one. */
  readonly copied = input<string | null>(null);
  /** A reading, not a value: nothing to paste anywhere. */
  readonly copyable = input(true);
  /** "256 bits": what the value is, under its name. */
  readonly meta = input('');
  /** Tinted, when this is the one that answers what was asked: a signature it matches. */
  readonly matched = input(false);

  protected readonly long = computed(() => this.value().length > LONG_RESULT);
}
