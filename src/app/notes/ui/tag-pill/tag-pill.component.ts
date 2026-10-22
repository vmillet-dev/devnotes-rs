import { ChangeDetectionStrategy, Component, input, output } from '@angular/core';

@Component({
  selector: 'app-tag-pill',
  templateUrl: './tag-pill.component.html',
  styleUrl: './tag-pill.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class TagPillComponent {
  readonly label = input.required<string>();
  readonly active = input(false);
  /** The notes it holds, on a facet's chip; `null` where a tag only names itself. */
  readonly count = input<number | null>(null);
  /** At `false` the tag is only a label. */
  readonly interactive = input(true);

  readonly toggled = output<string>();

  protected onClick(): void {
    this.toggled.emit(this.label());
  }
}
