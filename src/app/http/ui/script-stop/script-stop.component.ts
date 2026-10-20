import { ChangeDetectionStrategy, Component, computed, input, output } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { ScriptStop } from '@core/model/http.model';
import { TranslationRef } from '@core/services/i18n/translation-ref.model';

/**
 * A script that stopped: whose it was, what it said, on which line. Before the send it stands
 * in place of the answer; after it, above the answer that came.
 */
@Component({
  selector: 'app-script-stop',
  imports: [TranslocoPipe],
  templateUrl: './script-stop.component.html',
  styleUrl: './script-stop.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ScriptStopComponent {
  readonly stop = input.required<ScriptStop>();
  /** « Ouvrir le script »: not from the history, whose request may have changed since. */
  readonly openable = input(true);

  readonly opened = output<void>();

  protected readonly title = computed<TranslationRef>(() => {
    const stop = this.stop();
    if (stop.level === null || stop.level === undefined) return { key: 'http.scripts.stop.host' };
    return {
      key: `http.scripts.stop.${stop.script === 'before' ? 'before' : 'after'}.${stop.level}`,
      params: { name: stop.name ?? '' },
    };
  });
  /** What the script threw is its own words; the rest is said here. */
  protected readonly message = computed<TranslationRef | null>(() => {
    const kind = this.stop().kind ?? 'error';
    return kind === 'error' || kind === 'syntax' ? null : { key: `http.scripts.stop.kinds.${kind}` };
  });
  protected readonly where = computed<TranslationRef | null>(() => {
    const { line, script } = this.stop();
    const before = script === 'before';
    if (line === null || line === undefined) return before ? { key: 'http.scripts.stop.notSent' } : null;
    return { key: before ? 'http.scripts.stop.lineNotSent' : 'http.scripts.stop.line', params: { line } };
  });
}
