import { ChangeDetectionStrategy, Component, computed, inject, input, signal } from '@angular/core';
import { TranslocoPipe, TranslocoService } from '@jsverse/transloco';
import { ClipboardService } from '@core/services/clipboard/clipboard.service';
import { HttpSendStore } from '@core/services/http/http-send.store';
import { TranslationRef } from '@core/services/i18n/translation-ref.model';
import { ToolResult } from '@core/services/tools/tool.model';
import { SaveAsNoteDialogComponent } from '@app/save-as-note/save-as-note-dialog.component';
import { ResponseBodyComponent } from './response-body/response-body.component';
import { ResponseTimelineComponent } from './response-timeline/response-timeline.component';

type Section = 'body' | 'headers' | 'cookies' | 'timeline';

/** The lower half of a request's tab: what its last send answered. */
@Component({
  selector: 'app-response-pane',
  imports: [ResponseBodyComponent, ResponseTimelineComponent, SaveAsNoteDialogComponent, TranslocoPipe],
  templateUrl: './response-pane.component.html',
  styleUrl: './response-pane.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ResponsePaneComponent {
  readonly tabKey = input.required<string>();
  readonly requestName = input.required<string>();

  protected readonly sending = inject(HttpSendStore);
  private readonly transloco = inject(TranslocoService);
  private readonly clipboard = inject(ClipboardService);

  protected readonly sections: readonly Section[] = ['body', 'headers', 'cookies', 'timeline'];
  protected readonly section = signal<Section>('body');
  /** Taken on the click, like a tool's result: a send meanwhile does not change what is saved. */
  protected readonly saving = signal<ToolResult | null>(null);

  protected readonly state = computed(() => this.sending.states().get(this.tabKey()) ?? null);
  protected readonly phase = computed(() => this.state()?.phase ?? 'idle');
  protected readonly sendId = computed(() => this.state()?.sendId ?? '');
  protected readonly response = computed(() => {
    const state = this.state();
    return state?.phase === 'answered' ? state.response : null;
  });
  protected readonly failure = computed(() => {
    const state = this.state();
    return state?.phase === 'failed' ? state.notice.ref : null;
  });

  protected readonly counts = computed<Record<Section, number>>(() => ({
    body: 0,
    headers: this.response()?.headers.length ?? 0,
    cookies: this.response()?.cookies.length ?? 0,
    timeline: 0,
  }));

  /** `2` for a 2xx: the hue of the status. */
  protected readonly statusClass = computed(() => String(Math.floor((this.response()?.status ?? 0) / 100)));
  protected readonly hasText = computed(() => {
    const response = this.response();
    return response !== null && !response.binary && response.body !== '';
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

  protected copy(): void {
    const response = this.response();
    if (response !== null) void this.clipboard.copy(response.body);
  }

  protected saveAsNote(): void {
    const response = this.response();
    if (response === null || !this.hasText()) return;
    this.saving.set({
      title: { key: 'http.response.noteTitle', params: { name: this.requestName() } },
      kind: 'snippet',
      language: response.language,
      content: response.pretty ?? response.body,
    });
  }

  protected noteSource(): string {
    return `${this.transloco.translate('areas.http')} / ${this.requestName()}`;
  }

  protected saveFile(): void {
    void this.sending.saveBody(this.tabKey());
  }

  private format(value: number, digits: number): string {
    return new Intl.NumberFormat(this.transloco.activeLang(), { maximumFractionDigits: digits }).format(
      value,
    );
  }
}
