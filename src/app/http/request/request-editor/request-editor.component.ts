import {
  ChangeDetectionStrategy,
  Component,
  ResourceRef,
  Signal,
  computed,
  effect,
  inject,
  input,
  linkedSignal,
  output,
  resource,
  signal,
  untracked,
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
  HttpOrigin,
  ImpliedHeader,
  KeyValueRow,
  RequestAuth,
  RequestBodyDraft,
  SCRIPT_PHASES,
  ScriptParts,
  ScriptPhase,
  TransportSettings,
  WebsocketParts,
} from '@core/model/http.model';
import { UsedVariable, VariableRow } from '@core/model/http-environment.model';
import { HttpCollectionsStore } from '@core/services/http/http-collections.store';
import { HttpEnvironmentsStore } from '@core/services/http/http-environments.store';
import { HttpScriptFocusStore } from '@core/services/http/http-script-focus.store';
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
import { ProtectedQuestionComponent } from '@http/ui/protected-question/protected-question.component';
import { TransportEditorComponent } from '@http/ui/transport-editor/transport-editor.component';
import { UrlFieldComponent } from '../url-field/url-field.component';
import { VariableChipComponent } from '@http/ui/variable-chip/variable-chip.component';
import { CodeDialogComponent } from '@http/code-dialog/code-dialog.component';
import { ScriptEditorComponent } from '@http/ui/script-editor/script-editor.component';
import { VariablesTableComponent } from '@http/ui/variables-table/variables-table.component';

type Section =
  | 'params'
  | 'headers'
  | 'auth'
  | 'body'
  | 'query'
  | 'message'
  | 'protocols'
  | 'variables'
  | 'scripts'
  | 'settings';

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
    CodeDialogComponent,
    GraphqlEditorComponent,
    KeyValueTableComponent,
    ProtectedQuestionComponent,
    ScriptEditorComponent,
    SpinnerComponent,
    TranslocoPipe,
    TransportEditorComponent,
    UrlFieldComponent,
    VariableChipComponent,
    VariablesTableComponent,
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
  protected readonly environments = inject(HttpEnvironmentsStore);
  private readonly scriptFocus = inject(HttpScriptFocusStore);

  private readonly chosen = signal<Section>('params');
  protected readonly codeOpen = signal(false);
  protected readonly sections = computed<readonly Section[]>(
    () =>
      ({
        http: ['params', 'headers', 'auth', 'body', 'variables', 'scripts', 'settings'] as const,
        graphql: ['query', 'headers', 'auth', 'variables', 'scripts', 'settings'] as const,
        websocket: ['message', 'headers', 'auth', 'variables', 'protocols'] as const,
      })[this.tab().draft.kind],
  );
  /** The section chosen, or the first when the kind left it behind. */
  protected readonly section = computed<Section>(() =>
    this.sections().includes(this.chosen()) ? this.chosen() : this.sections()[0]!,
  );
  protected readonly headerNames = REQUEST_HEADERS;
  protected readonly scriptPhase = signal<ScriptPhase>('before');
  /** The line « Ouvrir le script » leads to. */
  protected readonly scriptLine = signal<number | null>(null);

  /** Methods only: what a request is — HTTP, GraphQL, a socket — is chosen when it is created. */
  protected readonly methods: readonly ChoiceOption[] = HTTP_METHODS.map((method) => ({
    id: method,
    name: method,
  }));

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

  /** What the variables resolve from: the request, its place, the environment and the sets. */
  private readonly variablesQuestion = computed(
    () => {
      const { draft, place } = this.tab();
      return {
        kind: draft.kind,
        parts: draft.parts,
        place,
        environmentId: this.environments.active()?.id ?? null,
        environments: this.environments.environments(),
        globals: this.environments.globals(),
        revision: this.collections.settingsRevision(),
      };
    },
    { equal: (a, b) => JSON.stringify(a) === JSON.stringify(b) },
  );
  private readonly usedResource = resource({
    params: () => this.variablesQuestion(),
    loader: ({ params }) => this.repository.variables(this.tab().draft, params.place, params.environmentId),
  });
  /** Each `{{name}}` the request holds, with where it resolves from: Rust's answer. */
  protected readonly used = this.lastOf<readonly UsedVariable[]>(this.usedResource, []);
  protected readonly scopes = computed(
    () => new Map(this.used().map((known) => [known.name, known.resolved?.scope ?? ('none' as const)])),
  );
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
    variables: this.tab().draft.parts.variables.filter((row) => row.enabled && row.name !== '').length,
    scripts: SCRIPT_PHASES.filter((phase) => this.tab().draft.parts.scripts[phase].trim() !== '').length,
    settings: Object.values(this.tab().draft.parts.transport).filter(
      (value) => value !== null && value !== undefined,
    ).length,
  }));

  constructor() {
    // « Ouvrir le script » under this tab's answer: its Scripts tab, on the line that stopped.
    effect(() => {
      const asked = this.scriptFocus.request();
      if (asked === null || asked.key !== this.tab().key) return;
      untracked(() => {
        this.chosen.set('scripts');
        this.scriptPhase.set(asked.focus.phase);
        this.scriptLine.set(asked.focus.line);
        this.scriptFocus.taken();
      });
    });
  }

  /** « par défaut », « d'après le corps », or the folder or collection that hands it down. */
  protected origin(header: ImpliedHeader): TranslationRef {
    return header.from === null
      ? { key: `http.headers.from.${header.source}` }
      : { key: `http.auth.from.${header.from.kind}`, params: { name: header.from.name } };
  }

  protected onMethod(method: string | null): void {
    if (method !== null) this.tabs.edit(this.tab().key, { method: method as HttpMethod });
  }

  protected choose(section: Section): void {
    this.chosen.set(section);
    this.scriptLine.set(null);
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

  protected onVariables(variables: readonly VariableRow[]): void {
    this.tabs.editParts(this.tab().key, { variables });
  }

  /** A name nothing sets, written into the active environment — or the globals — to be filled. */
  protected define(name: string): void {
    this.environments.define(name);
  }

  protected onScripts(scripts: ScriptParts): void {
    this.tabs.editParts(this.tab().key, { scripts });
  }

  /** A folder's or the collection's script, in their settings. */
  protected openLevel(from: HttpOrigin): void {
    this.collections.openSettingsOf(from.id, { phase: this.scriptPhase(), line: null });
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

  /** The yes a protected environment asked for, from the question under the line. */
  protected sendAnyway(): void {
    if (this.tab().draft.kind === 'websocket') void this.sockets.connect(this.tab(), true);
    else void this.sending.send(this.tab(), true);
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
