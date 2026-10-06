import { ChangeDetectionStrategy, Component, Signal, computed, inject, signal } from '@angular/core';
import { toSignal } from '@angular/core/rxjs-interop';
import { TranslocoPipe, TranslocoService } from '@jsverse/transloco';
import { ToolsRepository } from '@core/data/tools.repository';
import { CheckAnswer, CheckKind } from '@core/model/tool-answers.model';
import { liveResult } from '@core/services/tools/live-result';
import { Tool, ToolResult } from '@core/services/tools/tool.model';
import { toolState } from '@core/services/tools/tool-sessions';
import {
  Segment,
  SegmentedChoiceComponent,
} from '@shared/controls/segmented-choice/segmented-choice.component';
import { ResultRowComponent } from '@tools/ui/result-row/result-row.component';

type Luhn = Extract<CheckAnswer, { kind: 'luhn' }>;
type Iban = Extract<CheckAnswer, { kind: 'iban' }>;

const AUTO = 'auto';

const KINDS: readonly Segment[] = ([AUTO, 'luhn', 'iban'] as const).map((id) => ({
  id,
  labelKey: `tools.checks.kinds.${id}`,
}));

/** The published French example IBAN: valid, and nobody's account. */
const SAMPLE = 'FR76 3000 6000 0112 3456 7890 189';

/** Luhn and IBAN are Rust's; the page names the country in the language on screen. */
@Component({
  selector: 'app-checks-tool',
  imports: [ResultRowComponent, SegmentedChoiceComponent, TranslocoPipe],
  templateUrl: './checks-tool.component.html',
  styleUrl: './checks-tool.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ChecksToolComponent implements Tool {
  private readonly repository = inject(ToolsRepository);
  private readonly transloco = inject(TranslocoService);

  /** ⚠️ A card number is a secret: a plain signal, never a `toolState`, gone with the tool. */
  protected readonly text = signal('');
  protected readonly kind = toolState<CheckKind | null>('checks.kind', null);

  protected readonly kinds = KINDS;

  private readonly lang = toSignal(this.transloco.langChanges$, {
    initialValue: this.transloco.getActiveLang(),
  });

  protected readonly answer = liveResult(
    () => (this.text().trim() === '' ? undefined : { text: this.text(), kind: this.kind() }),
    (request) => this.repository.checkDigits(request),
  );

  protected readonly luhn = computed<Luhn | null>(() => {
    const answer = this.answer.value();
    return answer?.kind === 'luhn' ? answer : null;
  });

  protected readonly iban = computed<Iban | null>(() => {
    const answer = this.answer.value();
    return answer?.kind === 'iban' ? answer : null;
  });

  protected readonly problem = computed(() => {
    const answer = this.answer.value();
    return answer?.kind === 'unreadable' || answer?.kind === 'tooShort' ? answer : null;
  });

  /** "France", "Allemagne": the region's own name in the language on screen. */
  protected readonly countryName = computed(() => {
    const iban = this.iban();
    if (!iban) return null;
    return new Intl.DisplayNames([this.lang()], { type: 'region' }).of(iban.country) ?? iban.country;
  });

  /** « 27 caractères, conforme pour la France ». */
  protected readonly ibanLength = computed(() => {
    const iban = this.iban();
    if (!iban) return '';
    const country = iban.country;
    if (iban.verdict.kind === 'wrongLength') {
      return this.say('tools.checks.lengthWrong', {
        found: iban.verdict.found,
        expected: iban.verdict.expected,
        country,
      });
    }
    return iban.verdict.kind === 'unknownCountry'
      ? this.say('tools.checks.characters', { count: iban.length })
      : this.say('tools.checks.lengthRight', { count: iban.length, country });
  });

  /** « 14 · modulo 97 correct », or the digits the rest asks for. */
  protected readonly ibanKey = computed(() => {
    const iban = this.iban();
    if (!iban) return '';
    switch (iban.verdict.kind) {
      case 'valid':
      case 'wrongFormat':
        return this.say('tools.checks.ibanKeyRight', { given: iban.checkDigits });
      case 'wrongChecksum':
        return this.say('tools.checks.ibanKeyWrong', {
          given: iban.checkDigits,
          expected: iban.verdict.expected,
        });
      case 'wrongLength':
      case 'unknownCountry':
        return iban.checkDigits;
    }
  });

  /** ⚠️ Never: a card number or an IBAN must not end up in a note in clear, so there is nothing to save. */
  readonly result: Signal<ToolResult | null> = signal(null).asReadonly();

  sample(): void {
    this.kind.set(null);
    this.text.set(SAMPLE);
  }

  clear(): void {
    this.text.set('');
  }

  protected onInput(event: Event): void {
    this.text.set((event.target as HTMLInputElement).value);
  }

  protected lastDigit(grouped: string): string {
    return grouped.at(-1) ?? '';
  }

  /** The electronic format: the printed one without its spaces. */
  protected compact(printed: string): string {
    return printed.replaceAll(' ', '');
  }

  protected onKind(id: string): void {
    this.kind.set(id === AUTO ? null : (id as CheckKind));
  }

  private say(key: string, params: Record<string, unknown> = {}): string {
    this.lang();
    return this.transloco.translate(key, params);
  }
}
