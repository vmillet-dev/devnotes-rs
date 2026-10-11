import {
  ChangeDetectionStrategy,
  Component,
  ResourceRef,
  Signal,
  computed,
  inject,
  input,
  linkedSignal,
  output,
  resource,
  signal,
} from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { HttpRepository } from '@core/data/http.repository';
import {
  BodyAnswer,
  BodyKind,
  DEFAULT_TRANSPORT,
  GraphqlParts,
  HTTP_METHODS,
  HttpMethod,
  ImpliedHeader,
  KeyValueRow,
  RequestAuth,
  RequestBodyDraft,
  TransportSettings,
  WebsocketParts,
} from '@core/model/http.model';
import { HttpCollectionsStore } from '@core/services/http/http-collections.store';
import { HttpSendStore } from '@core/services/http/http-send.store';
import { HttpSocketsStore } from '@core/services/http/http-sockets.store';
import { HttpTabsStore, RequestTab } from '@core/services/http/http-tabs.store';
import { TranslationRef } from '@core/services/i18n/translation-ref.model';
import { ChoiceMenuComponent, ChoiceOption } from '@shared/controls/choice-menu/choice-menu.component';
import { SpinnerComponent } from '@shared/spinner/spinner.component';
import { HTTP_HEADERS } from '@tools/catalogue/http-headers/http-headers.data';
import { AuthEditorComponent } from '@http/ui/auth-editor/auth-editor.component';
import { BodyEditorComponent } from '../body-editor/body-editor.component';
import { GraphqlEditorComponent } from '../graphql-editor/graphql-editor.component';
import { WebsocketComposerComponent } from '../websocket-composer/websocket-composer.component';
import { KeyValueTableComponent } from '@http/ui/key-value-table/key-value-table.component';
import { TransportEditorComponent } from '@http/ui/transport-editor/transport-editor.component';
import { UrlFieldComponent } from '../url-field/url-field.component';

type Section = 'params' | 'headers' | 'auth' | 'body' | 'query' | 'message' | 'protocols' | 'settings';

/** The choices that turn a request into a GraphQL one or a socket, beside the methods. */
const GRAPHQL = 'GRAPHQL';
const WEBSOCKET = 'WEBSOCKET';

/** A header a request may send: the reference's own list, those a server alone writes left out. */
const REQUEST_HEADERS = HTTP_HEADERS.filter((header) => header.direction !== 'response').map(
  (header) => header.name,
);

const counted = (rows: readonly KeyValueRow[]) => rows.filter((row) => row.enabled && row.key !== '').length;

const sameFields = <T extends object>(a: T, b: T) =>
  (Object.keys(a) as (keyof T)[]).every((field) => a[field] === b[field]);

/** The open request: its method, its URL, and its parts a tab each. */
@Component({
  selector: 'app-request-editor',
  imports: [
    AuthEditorComponent,
    BodyEditorComponent,
    ChoiceMenuComponent,
    GraphqlEditorComponent,
    KeyValueTableComponent,
    SpinnerComponent,
    TranslocoPipe,
    TransportEditorComponent,
    UrlFieldComponent,
    WebsocketComposerComponent,
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
  private readonly sockets = inject(HttpSocketsStore);
  private readonly repository = inject(HttpRepository);
  private readonly collections = inject(HttpCollectionsStore);

  private readonly chosen = signal<Section>('params');
  protected readonly sections = computed<readonly Section[]>(
    () =>
      ({
        http: ['params', 'headers', 'auth', 'body', 'settings'] as const,
        graphql: ['query', 'headers', 'auth', 'settings'] as const,
        websocket: ['message', 'headers', 'auth', 'protocols'] as const,
      })[this.tab().draft.kind],
  );
  /** The section chosen, or the first when the kind left it behind. */
  protected readonly section = computed<Section>(() =>
    this.sections().includes(this.chosen()) ? this.chosen() : this.sections()[0]!,
  );
  protected readonly headerNames = REQUEST_HEADERS;

  protected readonly methods: readonly ChoiceOption[] = [
    ...HTTP_METHODS.map((method) => ({ id: method, name: method })),
    { id: GRAPHQL, name: 'GraphQL' },
    { id: WEBSOCKET, name: 'WebSocket' },
  ];
  protected readonly methodId = computed(() => {
    const draft = this.tab().draft;
    return { http: draft.method, graphql: GRAPHQL, websocket: WEBSOCKET }[draft.kind];
  });

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
  protected readonly transportAbove = computed(() => this.inherited()?.transport ?? DEFAULT_TRANSPORT);

  /** Asked with its method: a body is unusual on a `GET`, and that is Rust's to say. */
  private readonly bodyQuestion = computed(
    () => ({ body: this.tab().draft.parts.body, method: this.tab().draft.method }),
    { equal: sameFields },
  );
  private readonly bodyResource = resource({
    params: () => this.bodyQuestion(),
    loader: ({ params }) => this.repository.describeBody(params.body, params.method),
  });
  protected readonly bodyAnswer = this.lastOf<BodyAnswer | null>(this.bodyResource, null);

  /** What the headers sent unasked depend on: not the body's text, nor the description. */
  private readonly impliedQuestion = computed(
    () => {
      const { draft, place } = this.tab();
      const { url, headers, auth, body, graphql } = draft.parts;
      return {
        kind: draft.kind,
        url,
        headers,
        auth,
        body: body.kind,
        asGet: graphql.asGet,
        collectionId: place?.collectionId ?? null,
        folderId: place?.folderId ?? null,
        revision: this.collections.settingsRevision(),
      };
    },
    { equal: sameFields },
  );
  /** Rust composes the request, so Rust says what leaves with it: nothing is decided here. */
  private readonly impliedResource = resource({
    params: () => this.impliedQuestion(),
    loader: () => this.repository.impliedHeaders(this.tab().draft, this.tab().place),
  });
  protected readonly implied = this.lastOf<readonly ImpliedHeader[]>(this.impliedResource, []);
  protected readonly dirty = computed(() => this.tabs.isDirty(this.tab()));
  protected readonly inFlight = computed(
    () => this.sending.states().get(this.tab().key)?.phase === 'sending',
  );
  protected readonly socket = computed(() => this.sockets.states().get(this.tab().key) ?? null);
  /** Connected or on the way: the button disconnects. */
  protected readonly socketLive = computed(() => {
    const phase = this.socket()?.phase;
    return phase === 'open' || phase === 'connecting';
  });
  protected readonly counts = computed<Record<Section, number>>(() => ({
    params: counted(this.tab().draft.parts.params),
    headers: counted(this.tab().draft.parts.headers),
    auth: 0,
    body: 0,
    query: 0,
    message: this.tab().draft.parts.websocket.messages.length,
    protocols: this.tab().draft.parts.websocket.protocols.length,
    settings: Object.values(this.tab().draft.parts.transport).filter(
      (value) => value !== null && value !== undefined,
    ).length,
  }));

  /** « par défaut », « d'après le corps », or the folder or collection that hands it down. */
  protected origin(header: ImpliedHeader): TranslationRef {
    return header.from === null
      ? { key: `http.headers.from.${header.source}` }
      : { key: `http.auth.from.${header.from.kind}`, params: { name: header.from.name } };
  }

  protected onMethod(method: string | null): void {
    if (method === GRAPHQL) {
      this.tabs.edit(this.tab().key, { kind: 'graphql' });
    } else if (method === WEBSOCKET) {
      this.tabs.edit(this.tab().key, { kind: 'websocket' });
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

  protected onTransport(transport: TransportSettings): void {
    this.tabs.editParts(this.tab().key, { transport });
  }

  protected onBody(body: RequestBodyDraft): void {
    this.tabs.editParts(this.tab().key, { body });
  }

  protected onBodyKind(kind: BodyKind): void {
    this.tabs.chooseBodyKind(this.tab().key, kind);
  }

  protected send(): void {
    if (this.tab().draft.kind === 'websocket') {
      if (this.socketLive()) void this.sockets.close(this.tab().key);
      else void this.sockets.connect(this.tab());
    } else if (this.inFlight()) {
      void this.sending.cancel(this.tab().key);
    } else {
      void this.sending.send(this.tab());
    }
  }

  protected onWebsocket(websocket: WebsocketParts): void {
    this.tabs.editParts(this.tab().key, { websocket });
  }

  protected sendMessage(text: string): void {
    void this.sockets.send(this.tab().key, text);
  }

  protected onProtocols(event: Event): void {
    const protocols = (event.target as HTMLInputElement).value
      .split(',')
      .map((protocol) => protocol.trim())
      .filter((protocol) => protocol !== '');
    this.onWebsocket({ ...this.tab().draft.parts.websocket, protocols });
  }

  protected save(): void {
    if (this.tab().requestId === null) this.saveAsked.emit();
    else void this.tabs.save(this.tab().key);
  }

  /**
   * What Rust last answered for this tab, kept while the next answer is read: emptied between
   * two, what it draws — a list above a table, a line above the body, a mark on a tab — would
   * blink and push what is under it at each key. Another tab starts from `none`.
   */
  private lastOf<T>(answers: ResourceRef<T | undefined>, none: T): Signal<T> {
    return linkedSignal<{ key: string; fresh: T | undefined }, T>({
      source: () => ({ key: this.tab().key, fresh: answers.hasValue() ? answers.value() : undefined }),
      computation: ({ key, fresh }, previous) =>
        fresh ?? (previous?.source.key === key ? previous.value : none),
    });
  }
}
