import { ChangeDetectionStrategy, Component, computed, inject, signal } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { ToolsRepository } from '@core/data/tools.repository';
import { CharacterSet, PasswordRequest, Strength } from '@core/model/tool-answers.model';
import { liveResult } from '@core/services/tools/live-result';
import { Tool, ToolResult } from '@core/services/tools/tool.model';
import { toolState } from '@core/services/tools/tool-sessions';
import { OutputRowComponent } from '@tools/ui/output-row/output-row.component';

const SETS: readonly { readonly id: CharacterSet; readonly sample: string }[] = [
  { id: 'lowercase', sample: 'a–z' },
  { id: 'uppercase', sample: 'A–Z' },
  { id: 'digits', sample: '0–9' },
  { id: 'symbols', sample: '!#$%…' },
];

const STRENGTHS: readonly Strength[] = ['veryWeak', 'weak', 'fair', 'strong', 'veryStrong'];

const DEFAULTS: Omit<PasswordRequest, 'count'> = {
  length: 20,
  sets: ['lowercase', 'uppercase', 'digits', 'symbols'],
  avoidLookAlikes: true,
};

const DEFAULT_COUNT = 5;

/**
 * The options are remembered for the session, the passwords are not: they are the answer of a
 * `liveResult` that dies with the tool, and a trip elsewhere draws new ones.
 */
@Component({
  selector: 'app-password-tool',
  imports: [OutputRowComponent, TranslocoPipe],
  templateUrl: './password-tool.component.html',
  styleUrl: './password-tool.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class PasswordToolComponent implements Tool {
  private readonly repository = inject(ToolsRepository);

  protected readonly options = toolState<Omit<PasswordRequest, 'count'>>('password.options', DEFAULTS);
  protected readonly count = toolState('password.count', DEFAULT_COUNT);

  /** A new draw with the same options: part of the request, so the request changes. */
  private readonly draw = signal(0);

  protected readonly sets = SETS;
  protected readonly strengths = STRENGTHS;

  protected readonly answer = liveResult(
    () => ({ ...this.options(), count: this.count(), draw: this.draw() }),
    ({ draw: _draw, ...request }) => this.repository.generatePasswords(request),
  );

  protected readonly generated = computed(() => {
    const answer = this.answer.value();
    return answer?.kind === 'generated' ? answer : null;
  });

  /** An `f64` crosses as `number | null`: JSON has no NaN. */
  protected readonly bits = computed(() => Math.round(this.generated()?.entropyBits ?? 0));

  protected readonly level = computed(() => {
    const strength = this.generated()?.strength;
    return strength ? STRENGTHS.indexOf(strength) + 1 : 0;
  });

  readonly result = computed<ToolResult | null>(() => {
    const generated = this.generated();
    return generated
      ? {
          title: { key: 'tools.password.noteTitle' },
          kind: 'snippet',
          language: 'txt',
          content: generated.passwords.join('\n'),
          warning: { key: 'tools.password.saveWarning' },
        }
      : null;
  });

  /** Nothing typed to empty: Vider puts the options back as they were. */
  clear(): void {
    this.options.set(DEFAULTS);
    this.count.set(DEFAULT_COUNT);
  }

  protected regenerate(): void {
    this.draw.update((draw) => draw + 1);
  }

  protected isOn(set: CharacterSet): boolean {
    return this.options().sets.includes(set);
  }

  /** Kept in the order the sets are listed, whichever was ticked first. */
  protected onSet(set: CharacterSet, event: Event): void {
    const checked = (event.target as HTMLInputElement).checked;
    this.options.update((options) => ({
      ...options,
      sets: SETS.map(({ id }) => id).filter((id) => (id === set ? checked : options.sets.includes(id))),
    }));
  }

  protected onLookAlikes(event: Event): void {
    const checked = (event.target as HTMLInputElement).checked;
    this.options.update((options) => ({ ...options, avoidLookAlikes: checked }));
  }

  protected onLength(event: Event): void {
    const length = Number.parseInt((event.target as HTMLInputElement).value, 10);
    if (Number.isFinite(length)) {
      this.options.update((options) => ({ ...options, length }));
    }
  }

  protected onCount(event: Event): void {
    const count = Number.parseInt((event.target as HTMLInputElement).value, 10);
    if (Number.isFinite(count)) {
      this.count.set(count);
    }
  }
}
