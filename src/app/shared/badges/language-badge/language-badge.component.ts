import { ChangeDetectionStrategy, Component, computed, input } from '@angular/core';
import { LANGUAGE_LABELS, LanguageTag } from '@core/model/language.model';

@Component({
  selector: 'app-language-badge',
  templateUrl: './language-badge.component.html',
  styleUrl: './language-badge.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class LanguageBadgeComponent {
  readonly language = input.required<LanguageTag>();

  protected readonly label = computed(() => LANGUAGE_LABELS[this.language()]);
  protected readonly languageClass = computed(() => `lang-${this.language()}`);
}
