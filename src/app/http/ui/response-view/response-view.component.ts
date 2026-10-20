import {
  ChangeDetectionStrategy,
  Component,
  computed,
  inject,
  input,
  linkedSignal,
  output,
  signal,
} from '@angular/core';
import { TranslocoPipe, TranslocoService } from '@jsverse/transloco';
import { ScriptStop, SentResponse, TestResult } from '@core/model/http.model';
import { ClipboardService } from '@core/services/clipboard/clipboard.service';
import { TranslationRef } from '@core/services/i18n/translation-ref.model';
import { ToolResult } from '@core/services/tools/tool.model';
import { SaveAsNoteDialogComponent } from '@app/save-as-note/save-as-note-dialog.component';
import { ScriptStopComponent } from '@http/ui/script-stop/script-stop.component';
import { ResponseBodyComponent } from './response-body/response-body.component';
import { ResponseTestsComponent } from './response-tests/response-tests.component';
import { ResponseTimelineComponent } from './response-timeline/response-timeline.component';

type Section = 'body' | 'tests' | 'errors' | 'headers' | 'cookies' | 'timeline';

/** An answer read: its bar, then its body, headers, cookies and timeline. */
@Component({
  selector: 'app-response-view',
  imports: [
    ResponseBodyComponent,
    ResponseTestsComponent,
    ResponseTimelineComponent,
    SaveAsNoteDialogComponent,
    ScriptStopComponent,
    TranslocoPipe,
  ],
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
  /** The pane goes back to waiting for a send; the history keeps this one. */
  readonly clearAsked = output<void>();
  /** A script after the answer that stopped offers the way to it: not from the history. */
  readonly scriptOpenable = input(false);
  readonly scriptOpened = output<ScriptStop>();

  private readonly transloco = inject(TranslocoService);
  private readonly clipboard = inject(ClipboardService);

  /** A GraphQL answer's errors have a tab of their own, beside its data. */
  protected readonly sections = computed<readonly Section[]>(() => {
    const response = this.response();
    return [
      'body',
      ...((response.scripts?.tests?.length ?? 0) > 0 ? (['tests'] as const) : []),
      ...((response.graphql?.errors.length ?? 0) > 0 ? (['errors'] as const) : []),
      'headers',
      'cookies',
      'timeline',
    ];
  });
  protected readonly tests = computed(() => this.response().scripts?.tests ?? []);
  /** « 2/4 », red once one fails, green when all pass. */
  protected readonly testsRatio = computed(() => {
    const ran = this.tests().filter((test) => !test.skipped);
    const passed = ran.filter((test) => test.passed).length;
    return { text: `${passed}/${ran.length}`, failing: passed < ran.length };
  });
  /**
   * An answer that is errors and no data opens on them: on « Corps » it read `null`, the errors
   * waiting in a tab nothing pointed to. Any other answer keeps the tab that was being read.
   */
  private readonly chosen = linkedSignal<SentResponse, Section>({
    source: () => this.response(),
    computation: (response, previous) => {
      const graphql = response.graphql;
      if (graphql && graphql.errors.length > 0 && graphql.data === null) return 'errors';
      return previous === undefined || previous.value === 'errors' ? 'body' : previous.value;
    },
  });
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
    tests: 0,
    errors: this.response().graphql?.errors.length ?? 0,
    headers: this.response().headers.length,
    cookies: this.response().cookies.length,
    timeline: 0,
  }));

  /** A 200 that carries errors is no success: GraphQL servers answer so all the time. */
  protected readonly graphqlErrors = computed(() => this.response().graphql?.errors.length ?? 0);

  /** `2xx`, `4xx`…: what the badge's hue says, and what reads it when a colour cannot. */
  protected readonly family = computed(() => `${Math.floor(this.response().status / 100)}xx`);
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

  /** « Ligne 8 › » of a failed test: the script it was written in, on that line. */
  protected openTest(test: TestResult): void {
    this.scriptOpened.emit({
      kind: 'error',
      level: test.level ?? 'request',
      script: test.kind ?? 'tests',
      name: test.holder ?? null,
      owner: test.owner ?? null,
      line: test.line ?? null,
      message: test.message ?? '',
    });
  }
}
