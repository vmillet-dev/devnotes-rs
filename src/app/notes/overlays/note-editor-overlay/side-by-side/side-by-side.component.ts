import { ChangeDetectionStrategy, Component, input, output } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { SplitHandleComponent } from '@shared/layout/split-handle/split-handle.component';

/** Neither pane is dragged under this, in pixels. */
const CODE_FLOOR = 240;
const EXPLORER_FLOOR = 360;
/** Under this width the two are not squeezed side by side: the layout says it does not fit. */
export const SIDE_BY_SIDE_FROM = CODE_FLOOR + EXPLORER_FLOOR + 100;

/**
 * The code and the visualiser side by side, the line between them dragged or moved with the
 * arrows. It lays out what it is given — `[code]` on the left, `[explorer]` on the right — and
 * hands the share back to whoever keeps it.
 */
@Component({
  selector: 'app-side-by-side',
  imports: [SplitHandleComponent, TranslocoPipe],
  template: `
    <div class="code-pane" data-testid="side-by-side-code" [style.flex-basis.%]="share()">
      <ng-content select="[code]" />
    </div>
    <app-split-handle
      orientation="vertical"
      testId="side-by-side-handle"
      [share]="share()"
      [upperFloor]="codeFloor"
      [lowerFloor]="explorerFloor"
      [label]="'json.sideBySide.handle' | transloco"
      [valueText]="'json.sideBySide.share' | transloco: { share: share() }"
      (shareChanged)="shareChanged.emit($event)"
    />
    <div class="explorer-pane">
      <ng-content select="[explorer]" />
    </div>
  `,
  styleUrl: './side-by-side.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
  host: { 'data-testid': 'side-by-side' },
})
export class SideBySideComponent {
  /** The code's share of the width, in percent. */
  readonly share = input.required<number>();
  readonly shareChanged = output<number>();

  protected readonly codeFloor = CODE_FLOOR;
  protected readonly explorerFloor = EXPLORER_FLOOR;
}
