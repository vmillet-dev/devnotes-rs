import {
  ChangeDetectionStrategy,
  Component,
  computed,
  inject,
  input,
  output,
  resource,
  signal,
} from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { HttpRepository } from '@core/data/http.repository';
import {
  GraphqlParts,
  HTTP_METHODS,
  HttpMethod,
  KeyValueRow,
  RequestAuth,
  RequestBodyDraft,
  requestBadge,
} from '@core/model/http.model';
import { HttpCollectionsStore } from '@core/services/http/http-collections.store';
import { HttpSendStore } from '@core/services/http/http-send.store';
import { HttpTabsStore, RequestTab } from '@core/services/http/http-tabs.store';
import { ChoiceMenuComponent, ChoiceOption } from '@shared/controls/choice-menu/choice-menu.component';
import { HTTP_HEADERS } from '@tools/catalogue/http-headers/http-headers.data';
import { AuthEditorComponent } from '@http/ui/auth-editor/auth-editor.component';
import { BodyEditorComponent } from '../body-editor/body-editor.component';
import { GraphqlEditorComponent } from '../graphql-editor/graphql-editor.component';
import { KeyValueTableComponent } from '@http/ui/key-value-table/key-value-table.component';
import { UrlFieldComponent } from '../url-field/url-field.component';

type Section = 'params' | 'headers' | 'auth' | 'body' | 'query';

/** The choice that turns a request into a GraphQL one, beside the methods. */
const GRAPHQL = 'GRAPHQL';

/** A header a request may send: the reference's own list, those a server alone writes left out. */
const REQUEST_HEADERS = HTTP_HEADERS.filter((header) => header.direction !== 'response').map(
  (header) => header.name,
);

const counted = (rows: readonly KeyValueRow[]) => rows.filter((row) => row.enabled && row.key !== '').length;

/** The open request: its method, its URL, and its parts a tab each. */
@Component({
  selector: 'app-request-editor',
  imports: [
    AuthEditorComponent,
    BodyEditorComponent,
    ChoiceMenuComponent,
    GraphqlEditorComponent,
    KeyValueTableComponent,
    TranslocoPipe,
    UrlFieldComponent,
  ],
  templateUrl: './request-editor.component.html',
  styleUrl: './request-editor.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class RequestEditorComponent {
  readonly tab = input.required<RequestTab>();
  /** A request never saved asks where to go first: the page owns that dialog. */
  readonly saveAsked = output<void>();

  protected readonly tabs = inject(HttpTabsStore);
  protected readonly sending = inject(HttpSendStore);
  private readonly repository = inject(HttpRepository);
  private readonly collections = inject(HttpCollectionsStore);

  private readonly chosen = signal<Section>('params');
  protected readonly sections = computed<readonly Section[]>(() =>
    this.tab().draft.kind === 'graphql'
      ? ['query', 'headers', 'auth']
      : ['params', 'headers', 'auth', 'body'],
  );
  /** The section chosen, or the first when the kind left it behind. */
  protected readonly section = computed<Section>(() =>
    this.sections().includes(this.chosen()) ? this.chosen() : this.sections()[0]!,
  );
  protected readonly headerNames = REQUEST_HEADERS;

  protected readonly methods: readonly ChoiceOption[] = [
    ...HTTP_METHODS.map((method) => ({ id: method, name: method })),
    { id: GRAPHQL, name: 'GraphQL' },
  ];
  protected readonly methodId = computed(() =>
    this.tab().draft.kind === 'graphql' ? GRAPHQL : this.tab().draft.method,
  );

  protected readonly badge = computed(() => requestBadge(this.tab().draft));

  /** What the folders and the collection above hand down; nothing yet for a draft not placed. */
  private readonly inheritedResource = resource({
    params: () => {
      const place = this.tab().place;
      return place === null ? undefined : { ...place, revision: this.collections.settingsRevision() };
    },
    loader: ({ params }) => this.repository.inherited(params.collectionId, params.folderId),
  });
  protected readonly inherited = computed(() =>
    this.inheritedResource.hasValue() ? this.inheritedResource.value() : null,
  );

  private readonly bodyResource = resource({
    params: () => this.tab().draft.parts.body,
    loader: ({ params }) => this.repository.describeBody(params),
  });
  protected readonly bodyAnswer = computed(() =>
    this.bodyResource.hasValue() ? this.bodyResource.value() : null,
  );

  /** The `Content-Type` the body implies, sent unless a header sets one by hand. */
  protected readonly impliedType = computed(() => {
    const draft = this.tab().draft;
    const type =
      draft.kind === 'graphql'
        ? draft.parts.graphql.asGet
          ? null
          : 'application/json'
        : (this.bodyAnswer()?.contentType ?? null);
    const typed = this.tab().draft.parts.headers.some(
      (row) => row.enabled && row.key.toLowerCase() === 'content-type',
    );
    return typed ? null : type;
  });
  protected readonly dirty = computed(() => this.tabs.isDirty(this.tab()));
  protected readonly inFlight = computed(
    () => this.sending.states().get(this.tab().key)?.phase === 'sending',
  );
  protected readonly counts = computed<Record<Section, number>>(() => ({
    params: counted(this.tab().draft.parts.params),
    headers: counted(this.tab().draft.parts.headers),
    auth: 0,
    body: 0,
    query: 0,
  }));

  protected onMethod(method: string | null): void {
    if (method === GRAPHQL) {
      this.tabs.edit(this.tab().key, { kind: 'graphql' });
    } else if (method !== null) {
      this.tabs.edit(this.tab().key, { kind: 'http', method: method as HttpMethod });
    }
  }

  protected choose(section: Section): void {
    this.chosen.set(section);
  }

  protected onGraphql(graphql: GraphqlParts): void {
    this.tabs.editParts(this.tab().key, { graphql });
  }

  protected onName(event: Event): void {
    this.tabs.edit(this.tab().key, { name: (event.target as HTMLInputElement).value });
  }

  protected onUrl(url: string): void {
    void this.tabs.editUrl(this.tab().key, url);
  }

  protected onParams(params: readonly KeyValueRow[]): void {
    void this.tabs.editParams(this.tab().key, params);
  }

  protected onHeaders(headers: readonly KeyValueRow[]): void {
    this.tabs.editParts(this.tab().key, { headers });
  }

  protected onAuth(auth: RequestAuth): void {
    this.tabs.editParts(this.tab().key, { auth });
  }

  protected onBody(body: RequestBodyDraft): void {
    this.tabs.editParts(this.tab().key, { body });
  }

  protected send(): void {
    if (this.inFlight()) void this.sending.cancel(this.tab().key);
    else void this.sending.send(this.tab());
  }

  protected save(): void {
    if (this.tab().requestId === null) this.saveAsked.emit();
    else void this.tabs.save(this.tab().key);
  }
}
