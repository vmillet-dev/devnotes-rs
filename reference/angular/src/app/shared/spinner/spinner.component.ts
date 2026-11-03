import { ChangeDetectionStrategy, Component } from '@angular/core';

/**
 * Something is under way. Decorative: the sentence beside it is what a screen reader hears.
 * Its size is `--spinner-size` and its colour the text's. Reduced motion stops it turning,
 * from the one global block: what says "alive" then is whatever ticks beside it.
 */
@Component({
  selector: 'app-spinner',
  template: '',
  styleUrl: './spinner.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
  host: { 'aria-hidden': 'true' },
})
export class SpinnerComponent {}
