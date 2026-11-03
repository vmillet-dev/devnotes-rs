import { ChangeDetectionStrategy, Component, computed, input } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { visibleLines } from './visible-lines';

/** A text with what an editor hides: each line's ending, and the spaces left at its end. */
@Component({
  selector: 'app-line-view',
  imports: [TranslocoPipe],
  templateUrl: './line-view.component.html',
  styleUrl: './line-view.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class LineViewComponent {
  readonly text = input.required<string>();

  protected readonly view = computed(() => visibleLines(this.text()));
  protected readonly marks = { lf: '␊', crlf: '␍␊', cr: '␍' };
}
