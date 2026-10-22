import { ChangeDetectionStrategy, Component, input, output } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { TagPillComponent } from '@notes/ui/tag-pill/tag-pill.component';

@Component({
  selector: 'app-tag-rail',
  imports: [TagPillComponent, TranslocoPipe],
  templateUrl: './tag-rail.component.html',
  styleUrl: './tag-rail.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class TagRailComponent {
  readonly tags = input.required<readonly string[]>();
  readonly activeTags = input.required<ReadonlySet<string>>();
  /** The notes each tag holds in the space. */
  readonly counts = input<ReadonlyMap<string, number>>(new Map());

  readonly tagToggled = output<string>();
  /** Opens the corpus-wide manager: it acts on exactly what this rail shows. */
  readonly manageRequested = output<void>();
}
