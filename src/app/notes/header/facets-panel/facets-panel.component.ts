import { ChangeDetectionStrategy, Component, input, output } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { LanguageTag } from '@core/model/language.model';
import { LanguageRailComponent } from '@notes/header/language-rail/language-rail.component';
import { TagRailComponent } from '@notes/header/tag-rail/tag-rail.component';

/**
 * The tag rail and the language rail, shown only while they are wanted.
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
  imports: [TranslocoPipe, TagRailComponent, LanguageRailComponent],
  templateUrl: './facets-panel.component.html',
  styleUrl: './facets-panel.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class FacetsPanelComponent {
  readonly tags = input.required<readonly string[]>();
  readonly languages = input.required<readonly LanguageTag[]>();
  readonly activeTags = input.required<ReadonlySet<string>>();
  readonly activeLanguages = input.required<ReadonlySet<LanguageTag>>();
  /** The search and the quick filter count too: the way out clears all three. */
  readonly canClear = input(false);

  readonly tagToggled = output<string>();
  readonly languageToggled = output<LanguageTag>();
  readonly manageRequested = output<void>();
  readonly clearRequested = output<void>();
}
