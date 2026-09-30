import { ChangeDetectionStrategy, Component, computed, inject, signal } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { ToolsRepository } from '@core/data/tools.repository';
import { JwtAnswer } from '@core/model/tool-answers.model';
import { ClockService } from '@core/services/time/clock.service';
import { liveResult } from '@core/services/tools/live-result';
import { Tool, ToolResult } from '@core/services/tools/tool.model';
import { toolState } from '@core/services/tools/tool-sessions';
import { spanRef } from '@core/utils/relative-time.util';
import { IconComponent } from '@shared/icon/icon.component';
import { CopyValueComponent } from '@tools/ui/copy-value/copy-value.component';

type Decoded = Extract<JwtAnswer, { kind: 'decoded' }>;

/** Decoded and verified in Rust; how long ago or how far off is the page's, so it ages. */
@Component({
  selector: 'app-jwt-tool',
  imports: [CopyValueComponent, IconComponent, TranslocoPipe],
  templateUrl: './jwt-tool.component.html',
  styleUrl: './jwt-tool.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class JwtToolComponent implements Tool {
  private readonly repository = inject(ToolsRepository);
  private readonly clock = inject(ClockService);

  /** ⚠️ A token is a credential and a secret a secret: plain signals, gone with the tool. */
  protected readonly token = signal('');
  protected readonly secret = signal('');
  protected readonly secretShown = signal(false);
  protected readonly secretIsBase64 = toolState('jwt.secretIsBase64', false);

  protected readonly answer = liveResult(
    () =>
      this.token().trim() === ''
        ? undefined
        : { token: this.token(), secret: this.secret(), secretIsBase64: this.secretIsBase64() },
    (request) => this.repository.decodeJwt(request),
  );

  protected readonly decoded = computed<Decoded | null>(() => {
    const answer = this.answer.value();
    return answer?.kind === 'decoded' ? answer : null;
  });

  protected readonly malformed = computed(() => {
    const answer = this.answer.value();
    return answer?.kind === 'malformed' ? answer : null;
  });

  protected readonly dates = computed(() =>
    (this.decoded()?.dates ?? []).map((claim) => ({
      ...claim,
      relative: claim.epochMilliseconds === null ? null : spanRef(claim.epochMilliseconds, this.clock.now()),
    })),
  );

  /** "Expiré · il y a 3 jours": the claim the state rests on, as far off as it is now. */
  protected readonly stateWhen = computed(() => {
    const decoded = this.decoded();
    const claim = { expired: 'exp', valid: 'exp', notYetValid: 'nbf', undated: null }[
      decoded?.state ?? 'undated'
    ];
    return this.dates().find((date) => date.name === claim)?.relative ?? null;
  });

  readonly result = computed<ToolResult | null>(() => {
    const decoded = this.decoded();
    return decoded
      ? {
          title: { key: 'tools.jwt.noteTitle', params: { algorithm: decoded.algorithm ?? '' } },
          kind: 'snippet',
          language: 'json',
          content: decoded.document,
          warning: { key: 'tools.jwt.warning' },
        }
      : null;
  });

  clear(): void {
    this.token.set('');
    this.secret.set('');
    this.secretShown.set(false);
  }

  protected onToken(event: Event): void {
    this.token.set((event.target as HTMLTextAreaElement).value);
  }

  protected onSecret(event: Event): void {
    this.secret.set((event.target as HTMLInputElement).value);
  }

  protected onBase64(event: Event): void {
    this.secretIsBase64.set((event.target as HTMLInputElement).checked);
  }
}
