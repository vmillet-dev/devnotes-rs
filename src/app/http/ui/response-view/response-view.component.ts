import { ChangeDetectionStrategy, Component, computed, inject, input, output, signal } from '@angular/core';
import { TranslocoPipe, TranslocoService } from '@jsverse/transloco';
import { SentResponse } from '@core/model/http.model';
import { ClipboardService } from '@core/services/clipboard/clipboard.service';
import { TranslationRef } from '@core/services/i18n/translation-ref.model';
import { ToolResult } from '@core/services/tools/tool.model';
import { SaveAsNoteDialogComponent } from '@app/save-as-note/save-as-note-dialog.component';
import { ResponseBodyComponent } from './response-body/response-body.component';
import { ResponseTimelineComponent } from './response-timeline/response-timeline.component';

type Section = 'body' | 'errors' | 'headers' | 'cookies' | 'timeline';

/** An answer read: its bar, then its body, headers, cookies and timeline. */
@Component({
  selector: 'app-response-view',
  imports: [ResponseBodyComponent, ResponseTimelineComponent, SaveAsNoteDialogComponent, TranslocoPipe],
  templateUrl: './response-view.component.html',
  styleUrl: './response-view.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ResponseViewComponent {
  readonly response = input.required<SentResponse>();
  /** What Rust keeps the bytes under; `null` for an answer from the history, which kept none. */
  readonly sendId = input<string | null>(null);
  readonly requestName = input.required<string>();
  readonly fileAsked = output<void>();

  private readonly transloco = inject(TranslocoService);
  private readonly clipboard = inject(ClipboardService);

  /** A GraphQL answer's errors have a tab of their own, beside its data. */
  protected readonly sections = computed<readonly Section[]>(() =>
    (this.response().graphql?.errors.length ?? 0) > 0
      ? ['body', 'errors', 'headers', 'cookies', 'timeline']
      : ['body', 'headers', 'cookies', 'timeline'],
  );
  private readonly chosen = signal<Section>('body');
  protected readonly section = computed<Section>(() =>
    this.sections().includes(this.chosen()) ? this.chosen() : 'body',
  );
  /** What Corps lays out: a GraphQL answer's `data`, any other answer whole. */
  protected readonly shown = computed(() => {
    const response = this.response();
    return response.graphql ? { ...response, pretty: response.graphql.data ?? 'null' } : response;
  });
  /** Taken on the click, like a tool's result: a send meanwhile does not change what is saved. */
  protected readonly saving = signal<ToolResult | null>(null);

  protected readonly counts = computed<Record<Section, number>>(() => ({
    body: 0,
    errors: this.response().graphql?.errors.length ?? 0,
    headers: this.response().headers.length,
    cookies: this.response().cookies.length,
    timeline: 0,
  }));

  /** `2` for a 2xx: the hue of the status. */
  protected readonly statusClass = computed(() => String(Math.floor(this.response().status / 100)));
  protected readonly hasText = computed(() => !this.response().binary && this.response().body !== '');

  protected readonly millis = computed<TranslationRef>(() => ({
    key: 'http.response.millis',
    params: { value: this.format(this.response().millis, 0) },
  }));

  protected readonly size = computed<TranslationRef>(() => {
    const bytes = this.response().size;
    if (bytes < 1024) return { key: 'http.response.bytes', params: { value: this.format(bytes, 0) } };
    if (bytes < 1024 * 1024) {
      return { key: 'http.response.kilobytes', params: { value: this.format(bytes / 1024, 1) } };
    }
    return { key: 'http.response.megabytes', params: { value: this.format(bytes / 1024 / 1024, 1) } };
  });

  protected choose(section: Section): void {
    this.chosen.set(section);
  }

  protected copy(): void {
    void this.clipboard.copy(this.response().body);
  }

  protected saveAsNote(): void {
    const response = this.response();
    if (!this.hasText()) return;
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

  private format(value: number, digits: number): string {
    return new Intl.NumberFormat(this.transloco.activeLang(), { maximumFractionDigits: digits }).format(
      value,
    );
  }
}
