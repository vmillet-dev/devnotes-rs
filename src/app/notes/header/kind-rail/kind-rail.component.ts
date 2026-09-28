import { ChangeDetectionStrategy, Component, computed, input, output } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { FacetCount, NoteKind } from '@core/model/note.model';

/** The counts are the space's, from the back end: pressing one chip does not zero the others. */
@Component({
  selector: 'app-kind-rail',
  imports: [TranslocoPipe],
  templateUrl: './kind-rail.component.html',
  styleUrl: './kind-rail.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class KindRailComponent {
  readonly counts = input.required<readonly FacetCount<NoteKind>[]>();
  readonly activeKinds = input.required<ReadonlySet<NoteKind>>();

  readonly kindToggled = output<NoteKind>();
  readonly allChosen = output<void>();

  protected readonly total = computed(() => this.counts().reduce((sum, counted) => sum + counted.count, 0));
  protected readonly everyKind = computed(() => this.activeKinds().size === 0);
}
