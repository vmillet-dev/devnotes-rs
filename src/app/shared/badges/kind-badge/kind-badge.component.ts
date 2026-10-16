import { ChangeDetectionStrategy, Component, input } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { LanguageTag } from '@core/model/language.model';
import { NoteKind } from '@core/model/note.model';
import { LanguageBadgeComponent } from '../language-badge/language-badge.component';

/** A snippet names its format; a Note and a list say what they are. */
@Component({
  selector: 'app-kind-badge',
  imports: [TranslocoPipe, LanguageBadgeComponent],
  templateUrl: './kind-badge.component.html',
  styleUrl: './kind-badge.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class KindBadgeComponent {
  readonly kind = input.required<NoteKind>();
  readonly language = input.required<LanguageTag>();
}
