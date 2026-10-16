import { browser } from '@wdio/globals';
import { readFileSync, writeFileSync } from 'node:fs';

import { homeSpaceMarker } from './profile.js';
import { SCRIPT_TIMEOUT, SLOW_SCRIPT_TIMEOUT } from './timeouts.js';

import type {
  Attachment,
  Base64FileAnswer,
  DisplayNote,
  BoardQuery,
  BoardView,
  CheckAnswer,
  CheckRequest,
  CronAnswer,
  CronRequest,
  Folder,
  FolderColour,
  FolderDraft,
  ContainerSettings,
  HttpCollection,
  HttpContents,
  HttpFolder,
  HttpItem,
  HttpPlace,
  HttpRequest,
  HttpRequestDraft,
  HttpEnvironments,
  HistoryDay,
  HistoryEntry,
  Environment,
  EnvironmentPatch,
  HttpTree,
  NoteFiling,
  ExportReport,
  ImportReport,
  InstantAnswer,
  InstantRequest,
  JsonDiffAnswer,
  JsonDiffRequest,
  JsonQuery,
  JsonView,
  JwtAnswer,
  JwtRequest,
  LineBreaksAnswer,
  LineBreaksRequest,
  NoteDraft,
  NotePatch,
  NotePriority,
  NotesQuery,
  NotesView,
  PermissionsAnswer,
  PermissionsRequest,
  Priority,
  Registry,
  SizesAnswer,
  SizesRequest,
  Space,
  SpaceDraft,
  StatsRequest,
  TextDiffAnswer,
  TextDiffRequest,
  TextStats,
  TagUsage,
  TrashedNote,
  ZoneEntry,
  ZonesAnswer,
  ZonesRequest,
} from '@core/ipc/bindings';

/**
 * Seeding goes through the same bridge the application uses, into the same Rust. The
 * types come from the generated `bindings.ts`, so a Rust signature that moves stops this
 * compiling; that import is type-only and erased before `tsx` sees it.
 *
 * ⚠️ `window.__TAURI__` (exposed by `withGlobalTauri`) and not `browser.tauri.execute`,
 * which the service resolves through an HTTP endpoint it loses after a `reloadSession`.
 */
async function invoke<T>(command: string, args: Record<string, unknown> = {}): Promise<T> {
  const outcome = (await browser.executeAsync(
    (name: string, payload: Record<string, unknown>, done: (value: unknown) => void) => {
      const tauri = (window as unknown as Record<string, any>)['__TAURI__'];
      tauri.core
        .invoke(name, payload)
        .then((value: unknown) => done({ ok: value }))
        .catch((error: unknown) => done({ err: typeof error === 'string' ? error : JSON.stringify(error) }));
    },
    command,
    args,
  )) as { ok?: T; err?: string };

  if (outcome.err !== undefined) {
    throw new Error(`${command} failed: ${outcome.err}`);
  }
  return outcome.ok as T;
}

/** What derives a key: past `SCRIPT_TIMEOUT` on a slow runner. */
async function invokeSlowly<T>(command: string, args: Record<string, unknown>): Promise<T> {
  await browser.setTimeout({ script: SLOW_SCRIPT_TIMEOUT });
  try {
    return await invoke<T>(command, args);
  } finally {
    await browser.setTimeout({ script: SCRIPT_TIMEOUT });
  }
}

export const bridge = {
  /** Answers while locked: the registry is beside the libraries, not inside one. */
  listLibraries: () => invoke<Registry>('list_libraries'),
  /** Closes the open library and points the registry at this one: the front end then reloads. */
  openLibrary: (id: string) => invoke<null>('open_library', { id }),
  exploreJson: (query: JsonQuery) => invoke<JsonView>('explore_json', { query }),
  fixLineBreaks: (request: LineBreaksRequest) => invoke<LineBreaksAnswer>('fix_line_breaks', { request }),
  diffJson: (request: JsonDiffRequest) => invoke<JsonDiffAnswer>('diff_json', { request }),
  diffText: (request: TextDiffRequest) => invoke<TextDiffAnswer>('diff_text', { request }),
  describeInstant: (request: InstantRequest) => invoke<InstantAnswer>('describe_instant', { request }),
  placeInZones: (request: ZonesRequest) => invoke<ZonesAnswer>('place_in_zones', { request }),
  describeCron: (request: CronRequest) => invoke<CronAnswer>('describe_cron', { request }),
  convertSize: (request: SizesRequest) => invoke<SizesAnswer>('convert_size', { request }),
  describePermissions: (request: PermissionsRequest) =>
    invoke<PermissionsAnswer>('describe_permissions', { request }),
  checkDigits: (request: CheckRequest) => invoke<CheckAnswer>('check_digits', { request }),
  decodeJwt: (request: JwtRequest) => invoke<JwtAnswer>('decode_jwt', { request }),
  textStats: (request: StatsRequest) => invoke<TextStats>('text_stats', { request }),
  searchTimeZones: (query: string) => invoke<ZoneEntry[]>('search_time_zones', { query }),
  encodeBase64File: (path: string) => invoke<Base64FileAnswer>('encode_base64_file', { path }),
  httpTree: () => invoke<HttpTree>('http_tree'),
  createHttpCollection: (name: string) => invoke<HttpCollection>('create_http_collection', { name }),
  createHttpFolder: (collectionId: string, parentId: string | null, name: string) =>
    invoke<HttpFolder>('create_http_folder', { collectionId, parentId, name }),
  createHttpRequest: (draft: HttpRequestDraft) => invoke<HttpRequest>('create_http_request', { draft }),
  httpRequest: (id: string) => invoke<HttpRequest>('get_http_request', { id }),
  httpSettings: (item: HttpItem) => invoke<ContainerSettings>('http_settings', { item }),
  saveHttpSettings: (item: HttpItem, settings: ContainerSettings) =>
    invoke<null>('save_http_settings', { item, settings }),
  moveHttpItem: (item: HttpItem, place: HttpPlace) => invoke<null>('move_http_item', { item, place }),
  /** Answers how many entries went. */
  clearHttpHistory: () => invoke<number>('clear_http_history'),
  httpHistory: (tzOffsetMinutes: number) => invoke<HistoryDay[]>('http_history', { tzOffsetMinutes }),
  httpHistoryEntry: (id: string) => invoke<HistoryEntry>('http_history_entry', { id }),
  clearHttpCookies: () => invoke<number>('clear_http_cookies'),
  httpEnvironments: () => invoke<HttpEnvironments>('http_environments'),
  createHttpEnvironment: (name: string) => invoke<Environment>('create_http_environment', { name }),
  saveHttpEnvironment: (id: string, patch: EnvironmentPatch) =>
    invoke<Environment>('save_http_environment', { id, patch }),
  deleteHttpEnvironment: (id: string) => invoke<null>('delete_http_environment', { id }),
  countHttpContents: (item: HttpItem) => invoke<HttpContents>('count_http_contents', { item }),
  deleteHttpItem: (item: HttpItem) => invoke<null>('delete_http_item', { item }),
  listSpaces: () => invoke<Space[]>('list_spaces'),
  createSpace: (draft: SpaceDraft) => invoke<Space>('create_space', { draft }),
  renameSpace: (id: string, draft: SpaceDraft) => invoke<Space>('rename_space', { id, draft }),
  deleteSpace: (id: string, targetSpaceId: string) => invoke<null>('delete_space', { id, targetSpaceId }),

  listFolders: (spaceId: string | null = null) => invoke<Folder[]>('list_folders', { spaceId }),
  createFolder: (draft: FolderDraft) => invoke<Folder>('create_folder', { draft }),
  renameFolder: (id: string, name: string) => invoke<Folder>('rename_folder', { id, name }),
  recolourFolder: (id: string, colour: FolderColour) => invoke<Folder>('recolour_folder', { id, colour }),
  deleteFolder: (id: string) => invoke<null>('delete_folder', { id }),
  boardView: (query: BoardQuery) => invoke<BoardView>('board_view', { query }),
  /** `folderId` of `null` unfiles; the answer is what each note left, never a count. */
  fileNotes: (ids: string[], folderId: string | null) =>
    invoke<NoteFiling[]>('file_notes', { ids, folderId }),

  queryNotes: (query: NotesQuery) => invoke<NotesView>('query_notes', { query }),
  createNote: (draft: NoteDraft) => invoke<DisplayNote>('create_note', { draft }),
  updateNote: (id: string, patch: NotePatch) => invoke<DisplayNote>('update_note', { id, patch }),
  /** The whole body, as stored: a list sends a Note's words without their Markdown. */
  getNote: (id: string) => invoke<DisplayNote>('get_note', { id }),
  deleteNote: (id: string) => invoke<null>('delete_note', { id }),

  /** The batch commands answer with a count: a selection can hold an id that went stale. */
  deleteNotes: (ids: string[]) => invoke<number>('delete_notes', { ids }),
  moveNotes: (ids: string[], spaceId: string) => invoke<number>('move_notes', { ids, spaceId }),
  tagNotes: (ids: string[], tags: string[]) => invoke<number>('tag_notes', { ids, tags }),
  /** Answers each note whose priority moved, with the one it had: what the undo hands back. */
  setPriority: (ids: string[], priority: Priority) =>
    invoke<NotePriority[]>('set_priority', { ids, priority }),
  restorePriorities: (previous: NotePriority[]) => invoke<number>('restore_priorities', { previous }),
  purgeNotes: (ids: string[]) => invoke<number>('purge_notes', { ids }),
  emptyTrash: () => invoke<number>('empty_trash'),

  listTrash: (search = '') => invoke<TrashedNote[]>('list_trash', { search }),
  listTags: () => invoke<TagUsage[]>('list_tags'),
  renameTags: (tags: string[], into: string) => invoke<number>('rename_tags', { tags, into }),
  listAttachments: (noteId: string) => invoke<Attachment[]>('list_attachments', { noteId }),
  listGlobalPlaceholders: () => invoke<Record<string, string>>('list_global_placeholders'),
  setGlobalPlaceholders: (values: Record<string, string>) =>
    invoke<Record<string, string>>('set_global_placeholders', { values }),

  exportNotes: (path: string, spaceId: string | null = null, passphrase: string | null = null) =>
    invokeSlowly<ExportReport>('export_notes', {
      path,
      scope: spaceId === null ? { kind: 'library' } : { kind: 'space', spaceId },
      passphrase,
    }),
  importNotes: (path: string, passphrase: string | null = null) =>
    invokeSlowly<ImportReport>('import_notes', { path, passphrase }),
  exportIsProtected: (path: string) => invoke<boolean>('export_is_protected', { path }),

  changePassphrase: (current: string, next: string) =>
    invokeSlowly<null>('change_passphrase', { current, next }),
} as const;

/** A `NoteDraft` is exhaustive on the wire; a scenario cares about two or three fields. */
export function draft(overrides: Partial<NoteDraft> & Pick<NoteDraft, 'spaceId'>): NoteDraft {
  return {
    title: '',
    language: 'txt',
    content: '',
    source: '',
    tags: [],
    pinned: false,
    lifecycle: { kind: 'permanent' },
    ...overrides,
  };
}

/** Same idea for the query: the canvas always sends `pinnedFirst`, and `now` has to be live. */
export function query(overrides: Partial<NotesQuery> = {}): NotesQuery {
  return {
    spaceId: null,
    search: '',
    filter: 'all',
    tags: [],
    languages: [],
    kinds: [],
    priorities: [],
    now: new Date().toISOString(),
    tzOffsetMinutes: new Date().getTimezoneOffset(),
    pinnedFirst: true,
    ...overrides,
  };
}

/**
 * The space a first launch seeded, by id.
 *
 * Not `listSpaces()[0]`: spaces come back ordered by name, so a spec file that created `Ops`
 * would pick that one. `passTheGate()` records it the moment the samples land, while it is the
 * only space, in a file: each spec file runs in its own worker, where a module cache starts
 * empty.
 */
export async function rememberHomeSpace(): Promise<void> {
  const spaces = await bridge.listSpaces();
  const [only] = spaces;
  if (!only || spaces.length > 1) {
    throw new Error(
      `a first launch should seed one space, found ${JSON.stringify(spaces.map((space) => space.name))}`,
    );
  }

  writeFileSync(homeSpaceMarker(), only.id, 'utf8');
}

export async function homeSpaceId(): Promise<string> {
  try {
    const remembered = readFileSync(homeSpaceMarker(), 'utf8').trim();
    if (remembered) return remembered;
  } catch {
    // Not written yet: said below.
  }

  throw new Error('no first launch recorded its space in this run: open the canvas before asking for it');
}
