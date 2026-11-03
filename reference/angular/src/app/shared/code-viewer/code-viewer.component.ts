import { ChangeDetectionStrategy, Component, computed, input } from '@angular/core';
import { FALLBACK_LANGUAGE, LanguageTag } from '@core/model/language.model';
import { grammarOf } from '@core/services/syntax/grammars';
import { highlightLines } from '@core/services/syntax/highlighter';

@Component({
  selector: 'app-code-viewer',
  templateUrl: './code-viewer.component.html',
  styleUrl: './code-viewer.component.scss',
  host: { '[class.compact]': 'compact()' },
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class CodeViewerComponent {
  readonly content = input.required<string>();
  readonly language = input<LanguageTag>(FALLBACK_LANGUAGE);

  readonly showLineNumbers = input(true);

  /** The embedded variant: no margin, no scrolling, no font size of its own. */
  readonly compact = input(false);

  protected readonly lines = computed<readonly string[]>(() =>
    highlightLines(this.content(), grammarOf(this.language())),
  );
}
