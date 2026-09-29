import { ChangeDetectionStrategy, Component, input, output } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { LanguageTag } from '@core/model/language.model';
import { FacetCount, NoteKind, Priority } from '@core/model/note.model';
import { KindRailComponent } from '@notes/header/kind-rail/kind-rail.component';
import { PriorityRailComponent } from '@notes/header/priority-rail/priority-rail.component';
import { LanguageRailComponent } from '@notes/header/language-rail/language-rail.component';
import { TagRailComponent } from '@notes/header/tag-rail/tag-rail.component';

/**
 * The kind, priority, tag and language rails, shown only while they are wanted.
 *
 * They were two permanent 44px bands — 92px of a 720px window spent on a secondary
 * filter, and the largest single reason the first card started 39% of the way down. They
 * are **facets**: worth reaching, not worth a band each.
 *
 * Its trigger lives in the topbar and not here: the two sit in different rows, and a
 * component cannot be in two places. `notes-header` owns the disclosure state for that
 * reason, and forces it open whenever a facet is selected — a filter you cannot see is a
 * filter you cannot undo.
 */
@Component({
  selector: 'app-facets-panel',
  imports: [TranslocoPipe, KindRailComponent, PriorityRailComponent, TagRailComponent, LanguageRailComponent],
  templateUrl: './facets-panel.component.html',
  styleUrl: './facets-panel.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class FacetsPanelComponent {
  readonly tags = input.required<readonly string[]>();
  readonly languages = input.required<readonly LanguageTag[]>();
  readonly activeTags = input.required<ReadonlySet<string>>();
  readonly activeLanguages = input.required<ReadonlySet<LanguageTag>>();
  readonly kindCounts = input.required<readonly FacetCount<NoteKind>[]>();
  readonly activeKinds = input.required<ReadonlySet<NoteKind>>();
  readonly priorityCounts = input.required<readonly FacetCount<Priority>[]>();
  readonly activePriorities = input.required<ReadonlySet<Priority>>();
  /** The search counts too: the way out clears it with the rails. */
  readonly canClear = input(false);

  readonly tagToggled = output<string>();
  readonly languageToggled = output<LanguageTag>();
  readonly kindToggled = output<NoteKind>();
  readonly allKindsChosen = output<void>();
  readonly priorityToggled = output<Priority>();
  readonly allPrioritiesChosen = output<void>();
  readonly manageRequested = output<void>();
  readonly clearRequested = output<void>();
}
