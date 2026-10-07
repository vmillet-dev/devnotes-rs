import { ChangeDetectionStrategy, Component, computed, inject, input } from '@angular/core';
import { TranslocoPipe, TranslocoService } from '@jsverse/transloco';
import { HttpSendStore, contentTypeOf } from '@core/services/http/http-send.store';
import { TranslationRef } from '@core/services/i18n/translation-ref.model';

/** The lower half of a request's tab: what its last send answered. */
@Component({
  selector: 'app-response-pane',
  imports: [TranslocoPipe],
  templateUrl: './response-pane.component.html',
  styleUrl: './response-pane.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ResponsePaneComponent {
  readonly tabKey = input.required<string>();

  protected readonly sending = inject(HttpSendStore);
  private readonly transloco = inject(TranslocoService);

  protected readonly state = computed(() => this.sending.states().get(this.tabKey()) ?? null);
  protected readonly phase = computed(() => this.state()?.phase ?? 'idle');
  protected readonly response = computed(() => {
    const state = this.state();
    return state?.phase === 'answered' ? state.response : null;
  });
  protected readonly failure = computed(() => {
    const state = this.state();
    return state?.phase === 'failed' ? state.notice.ref : null;
  });

  /** `2` for a 2xx: the hue of the status. */
  protected readonly statusClass = computed(() => {
    const status = this.response()?.status ?? 0;
    return String(Math.floor(status / 100));
  });
  protected readonly type = computed(() => {
    const response = this.response();
    return response === null ? '' : (contentTypeOf(response)?.split(';')[0]?.trim() ?? '?');
  });

  protected readonly millis = computed<TranslationRef | null>(() => {
    const response = this.response();
    return response === null
      ? null
      : { key: 'http.response.millis', params: { value: this.format(response.millis, 0) } };
  });

  protected readonly size = computed<TranslationRef | null>(() => {
    const bytes = this.response()?.size;
    if (bytes === undefined) return null;
    if (bytes < 1024) return { key: 'http.response.bytes', params: { value: this.format(bytes, 0) } };
    if (bytes < 1024 * 1024) {
      return { key: 'http.response.kilobytes', params: { value: this.format(bytes / 1024, 1) } };
    }
    return { key: 'http.response.megabytes', params: { value: this.format(bytes / 1024 / 1024, 1) } };
  });

  protected save(): void {
    void this.sending.saveBody(this.tabKey());
  }

  private format(value: number, digits: number): string {
    return new Intl.NumberFormat(this.transloco.activeLang(), { maximumFractionDigits: digits }).format(
      value,
    );
  }
}
