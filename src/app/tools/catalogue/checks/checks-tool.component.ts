import { ChangeDetectionStrategy, Component, computed, inject, signal } from '@angular/core';
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
import { OutputRowComponent } from '@tools/ui/output-row/output-row.component';

type Luhn = Extract<CheckAnswer, { kind: 'luhn' }>;
type Iban = Extract<CheckAnswer, { kind: 'iban' }>;

const AUTO = 'auto';

const KINDS: readonly Segment[] = ([AUTO, 'luhn', 'iban'] as const).map((id) => ({
  id,
  labelKey: `tools.checks.kinds.${id}`,
}));

/** Every digit but the last four hidden: what a note keeps of a card number. */
export function masked(grouped: string): string {
  const total = [...grouped].filter((character) => /\d/.test(character)).length;
  let seen = 0;
  return [...grouped]
    .map((character) => {
      if (!/\d/.test(character)) return character;
      seen += 1;
      return seen > total - 4 ? character : '•';
    })
    .join('');
}

/** The published French example IBAN: valid, and nobody's account. */
const SAMPLE = 'FR76 3000 6000 0112 3456 7890 189';

/** Luhn and IBAN are Rust's; the page names the country in the language on screen. */
@Component({
  selector: 'app-checks-tool',
  imports: [OutputRowComponent, SegmentedChoiceComponent, TranslocoPipe],
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

  readonly result = computed<ToolResult | null>(() => {
    this.lang();
    const t = (key: string, params?: Record<string, unknown>) =>
      this.transloco.translate<string>(key, params);
    const luhn = this.luhn();
    if (luhn) {
      const verdict = luhn.valid ? t('tools.checks.valid') : t('tools.checks.invalid');
      const network = luhn.network ? ` · ${t(`tools.checks.networks.${luhn.network}`)}` : '';
      const shown = masked(luhn.grouped);
      return {
        title: { key: 'tools.checks.cardNoteTitle', params: { last: shown.replace(/\D/g, '') } },
        kind: 'snippet',
        language: 'txt',
        content: `${shown}\n${verdict} (Luhn)${network}`,
      };
    }
    const iban = this.iban();
    return iban
      ? {
          title: { key: 'tools.checks.ibanNoteTitle', params: { country: iban.country } },
          kind: 'snippet',
          language: 'txt',
          content: `${iban.printed}\n${iban.verdict.kind === 'valid' ? t('tools.checks.valid') : t('tools.checks.invalid')}`,
        }
      : null;
  });

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

  protected onKind(id: string): void {
    this.kind.set(id === AUTO ? null : (id as CheckKind));
  }
}
