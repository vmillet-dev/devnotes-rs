import { ChangeDetectionStrategy, Component, input, output } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { LanguageTag } from '@core/model/language.model';
import { LanguageBadgeComponent } from '@shared/badges/language-badge/language-badge.component';

/** The languages come from the back end: a facet that would filter nothing is noise. */
@Component({
  selector: 'app-language-rail',
  imports: [LanguageBadgeComponent, TranslocoPipe],
  templateUrl: './language-rail.component.html',
  styleUrl: './language-rail.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class LanguageRailComponent {
  readonly languages = input.required<readonly LanguageTag[]>();
  readonly activeLanguages = input.required<ReadonlySet<LanguageTag>>();

  readonly languageToggled = output<LanguageTag>();
}
