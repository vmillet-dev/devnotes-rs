import { browser } from '@wdio/globals';
import { readFileSync, writeFileSync } from 'node:fs';

import { homeSpaceMarker } from './profile.js';

import type {
  Attachment,
  DisplayNote,
  BoardQuery,
  BoardView,
  Folder,
  FolderColour,
  FolderDraft,
  NoteFiling,
  ExportReport,
  ImportReport,
  NoteDraft,
  NotePatch,
  NotesQuery,
  NotesView,
  Registry,
  Space,
  SpaceDraft,
  TagUsage,
  TrashedNote,
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

export const bridge = {
  /** ⚠️ Answers while locked: the registry is beside the libraries, not inside one. */
  listLibraries: () => invoke<Registry>('list_libraries'),
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
  deleteNote: (id: string) => invoke<null>('delete_note', { id }),

  /** The batch commands answer with a count: a selection can hold an id that went stale. */
  deleteNotes: (ids: string[]) => invoke<number>('delete_notes', { ids }),
  moveNotes: (ids: string[], spaceId: string) => invoke<number>('move_notes', { ids, spaceId }),
  tagNotes: (ids: string[], tags: string[]) => invoke<number>('tag_notes', { ids, tags }),
  purgeNotes: (ids: string[]) => invoke<number>('purge_notes', { ids }),
  emptyTrash: () => invoke<number>('empty_trash'),

  listTrash: () => invoke<TrashedNote[]>('list_trash'),
  listTags: () => invoke<TagUsage[]>('list_tags'),
  renameTags: (tags: string[], into: string) => invoke<number>('rename_tags', { tags, into }),
  listAttachments: (noteId: string) => invoke<Attachment[]>('list_attachments', { noteId }),
  listGlobalPlaceholders: () => invoke<Record<string, string>>('list_global_placeholders'),
  setGlobalPlaceholders: (values: Record<string, string>) =>
    invoke<Record<string, string>>('set_global_placeholders', { values }),

  exportNotes: (path: string, spaceId: string | null = null, passphrase: string | null = null) =>
    invoke<ExportReport>('export_notes', {
      path,
      scope: spaceId === null ? { kind: 'library' } : { kind: 'space', spaceId },
      passphrase,
    }),
  importNotes: (path: string, passphrase: string | null = null) =>
    invoke<ImportReport>('import_notes', { path, passphrase }),
  exportIsProtected: (path: string) => invoke<boolean>('export_is_protected', { path }),

  changePassphrase: (current: string, next: string) => invoke<null>('change_passphrase', { current, next }),
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
    now: new Date().toISOString(),
    tzOffsetMinutes: new Date().getTimezoneOffset(),
    pinnedFirst: true,
    ...overrides,
  };
}

/**
 * The space the install seeded, by id, for the whole run.
 *
 * ⚠️ Not `listSpaces()[0]`: `list_spaces` orders by `name COLLATE NOCASE`, so the first
 * row is the alphabetically first space and a spec file that created `Ops` would pick
 * that one instead. Resolved once while a virgin profile still holds exactly one space,
 * then read back from a file — each spec file gets its own worker process, so a
 * module-level cache would be empty again in the next one.
 */
function readHomeSpaceId(): string | null {
  try {
    return readFileSync(homeSpaceMarker(), 'utf8').trim() || null;
  } catch {
    return null;
  }
}

export async function homeSpaceId(): Promise<string> {
  const remembered = readHomeSpaceId();
  if (remembered) {
    return remembered;
  }

  const spaces = await bridge.listSpaces();
  const first = spaces[0];
  if (!first) {
    throw new Error('a fresh profile should have seeded one space');
  }
  if (spaces.length > 1) {
    throw new Error(
      `the seeded space is asked for with ${spaces.length} spaces present, so "the first one" is a ` +
        `guess. It has to be resolved while the profile is still virgin, in 01-first-launch. ` +
        `Found ${JSON.stringify(spaces.map((space) => space.name))}`,
    );
  }

  writeFileSync(homeSpaceMarker(), first.id, 'utf8');
  return first.id;
}
