import {
  ChangeDetectionStrategy,
  Component,
  computed,
  effect,
  inject,
  input,
  linkedSignal,
  resource,
  untracked,
} from '@angular/core';
import { TranslocoPipe, TranslocoService } from '@jsverse/transloco';
import { HttpRepository } from '@core/data/http.repository';
import { SentResponse } from '@core/model/http.model';
import { LanguageTag } from '@core/model/language.model';
import { ClipboardService } from '@core/services/clipboard/clipboard.service';
import { contentTypeOf } from '@core/services/http/http-send.store';
import { JsonExplorerStore } from '@core/state/json-explorer.store';
import { CodeViewerComponent } from '@shared/code-viewer/code-viewer.component';
import {
  Segment,
  SegmentedChoiceComponent,
} from '@shared/controls/segmented-choice/segmented-choice.component';
import { JsonExplorerComponent } from '@shared/json-explorer/json-explorer.component';

export type BodyView = 'pretty' | 'raw' | 'graph' | 'tree';

/** Past this, the text is laid out but not coloured: highlighting a megabyte stalls the page. */
const HIGHLIGHT_LIMIT = 256 * 1024;

const segment = (id: BodyView): Segment => ({ id, labelKey: `http.response.views.${id}` });
const JSON_VIEWS = (['pretty', 'raw', 'graph', 'tree'] as const).map(segment);
const TEXT_VIEWS = (['pretty', 'raw'] as const).map(segment);

/** A response's body: laid out and coloured, as it came, as a graph or a tree, or an image. */
@Component({
  selector: 'app-response-body',
  imports: [CodeViewerComponent, JsonExplorerComponent, SegmentedChoiceComponent, TranslocoPipe],
  templateUrl: './response-body.component.html',
  styleUrl: './response-body.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
  providers: [JsonExplorerStore],
})
export class ResponseBodyComponent {
  readonly response = input.required<SentResponse>();
  /** What Rust keeps the bytes under, for an image. */
  readonly sendId = input.required<string>();

  protected readonly explorer = inject(JsonExplorerStore);
  private readonly repository = inject(HttpRepository);
  private readonly clipboard = inject(ClipboardService);
  private readonly transloco = inject(TranslocoService);

  protected readonly locale = computed(() => this.transloco.activeLang());
  protected readonly isJson = computed(() => this.response().language === 'json');
  protected readonly views = computed(() => (this.isJson() ? JSON_VIEWS : TEXT_VIEWS));
  /** Formatted first, and back to it when a body arrives that has no graph. */
  protected readonly view = linkedSignal<readonly Segment[], BodyView>({
    source: this.views,
    computation: (views, previous) =>
      previous && views.some((each) => each.id === previous.value) ? previous.value : 'pretty',
  });

  protected readonly shown = computed(() => {
    const response = this.response();
    return this.view() === 'pretty' ? (response.pretty ?? response.body) : response.body;
  });
  protected readonly language = computed<LanguageTag>(() =>
    this.view() === 'pretty' && this.shown().length <= HIGHLIGHT_LIMIT ? this.response().language : 'txt',
  );
  protected readonly type = computed(() => contentTypeOf(this.response())?.split(';')[0]?.trim() ?? '?');

  private readonly imageResource = resource({
    params: () => {
      const response = this.response();
      const image = response.binary && this.type().toLowerCase().startsWith('image/');
      return image ? { id: this.sendId(), size: response.size } : undefined;
    },
    loader: ({ params }) => this.repository.responseImage(params.id),
  });
  protected readonly image = computed(() =>
    this.imageResource.hasValue() ? (this.imageResource.value() ?? null) : null,
  );

  constructor() {
    // Explored only once asked: most JSON answers are read, not walked.
    effect(() => {
      const view = this.view();
      const body = this.response().body;
      if (view === 'graph' || view === 'tree') untracked(() => this.explorer.load(body));
    });
  }

  protected choose(id: string): void {
    this.view.set(id as BodyView);
  }

  protected copy(text: string): void {
    void this.clipboard.copy(text);
  }
}
