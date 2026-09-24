/**
 * What generation cannot cover: JSON has no date type, so every `Date` arrives and
 * leaves as an ISO 8601 string. The conversions spread the wire object and override
 * only the dates, so a scalar added on the Rust side costs nothing here.
 */
import type {
  Attachment as WireAttachment,
  DisplayNote as WireNote,
  NoteDraft as WireNoteDraft,
  NoteFooter as WireNoteFooter,
  NoteLifecycle as WireNoteLifecycle,
  NotePatch as WireNotePatch,
  NoteSection as WireNoteSection,
  NotesQuery as WireNotesQuery,
  NotesView as WireNotesView,
  TrashedNote as WireTrashedNote,
} from '@core/ipc/bindings';
import {
  Attachment,
  Note,
  NoteDraft,
  NoteFooter,
  NoteLifecycle,
  NotePatch,
  NoteSection,
  NotesQuery,
  NotesView,
  TrashedNote,
} from '@core/model/note.model';

export class ContractError extends Error {
  constructor(field: string, value: unknown) {
    super(`Broken contract: field "${field}" is unusable (${JSON.stringify(value)})`);
    this.name = 'ContractError';
  }
}

function parseIsoDate(value: string, field: string): Date {
  const date = new Date(value);
  // Fail loudly rather than let an `Invalid Date` surface as `NaN` in the labels.
  if (Number.isNaN(date.getTime())) {
    throw new ContractError(field, value);
  }
  return date;
}

/** `toISOString()` throws a bare `RangeError`, without saying which field is at fault. */
export function toIsoString(date: Date, field: string): string {
  if (Number.isNaN(date.getTime())) {
    throw new ContractError(field, date);
  }
  return date.toISOString();
}

/** Drops the keys a patch does not carry, which serde reads as "do not touch". */
function withoutUndefined<T extends object>(source: T): T {
  const kept = {} as T;

  for (const key of Object.keys(source) as (keyof T)[]) {
    const value = source[key];
    if (value !== undefined) {
      kept[key] = value;
    }
  }

  return kept;
}

function toLifecycle(dto: WireNoteLifecycle): NoteLifecycle {
  return dto.kind === 'expires'
    ? { kind: 'expires', at: parseIsoDate(dto.at, 'lifecycle.at') }
    : { kind: 'permanent' };
}

function toWireLifecycle(lifecycle: NoteLifecycle): WireNoteLifecycle {
  return lifecycle.kind === 'expires'
    ? { kind: 'expires', at: toIsoString(lifecycle.at, 'lifecycle.at') }
    : { kind: 'permanent' };
}

function toFooter(dto: WireNoteFooter): NoteFooter {
  switch (dto.kind) {
    case 'source':
      return { kind: 'source', value: dto.value };
    case 'expiry':
      return { kind: 'expiry', at: parseIsoDate(dto.at, 'footer.at') };
    case 'age':
      return { kind: 'age', at: parseIsoDate(dto.at, 'footer.at') };
    default:
      // Only an older front end against a newer back end: exhaustive at compile time.
      throw new ContractError('footer.kind', (dto satisfies never as { kind: string }).kind);
  }
}

/**
 * `placeholderValues` is dropped on purpose: the front reads what was typed through
 * `placeholders[].value`, already paired with the field the text carries. A second,
 * unpaired copy would invite reading a value whose token has left the content.
 */
export function toNote({ placeholderValues: _stored, ...dto }: WireNote): Note {
  return {
    ...dto,
    createdAt: parseIsoDate(dto.createdAt, 'createdAt'),
    updatedAt: parseIsoDate(dto.updatedAt, 'updatedAt'),
    lifecycle: toLifecycle(dto.lifecycle),
    footer: toFooter(dto.footer),
    // Declared optional by `#[serde(default)]`, which keeps older export files readable.
    kind: dto.kind ?? 'snippet',
    items: dto.items ?? [],
    // `#[specta(optional)]`, so an absent key means unfiled rather than untouched here.
    folderId: dto.folderId ?? null,
  };
}

export function toTrashedNote(dto: WireTrashedNote): TrashedNote {
  return {
    id: dto.id,
    spaceId: dto.spaceId,
    title: dto.title,
    language: dto.language,
    content: dto.content,
    tags: [...dto.tags],
    deletedAt: parseIsoDate(dto.deletedAt, 'deletedAt'),
    purgeAt: parseIsoDate(dto.purgeAt, 'purgeAt'),
    kind: dto.kind ?? 'snippet',
  };
}

export function toAttachment(dto: WireAttachment): Attachment {
  return { ...dto, createdAt: parseIsoDate(dto.createdAt, 'createdAt') };
}

export function toWireNoteDraft(draft: NoteDraft): WireNoteDraft {
  return {
    ...draft,
    tags: [...draft.tags],
    items: [...draft.items],
    lifecycle: toWireLifecycle(draft.lifecycle),
  };
}

/**
 * ⚠️ A key left out is a field the patch does not touch; a key sent as `null` would
 * overwrite it — hence the filtering, and `#[specta(optional)]` on the Rust side.
 */
export function toWireNotePatch(patch: NotePatch): WireNotePatch {
  const { lifecycle, tags, items, ...scalars } = patch;
  const dto: WireNotePatch = withoutUndefined(scalars);

  if (lifecycle !== undefined) dto.lifecycle = toWireLifecycle(lifecycle);
  if (tags !== undefined) dto.tags = [...tags];
  if (items !== undefined) dto.items = items.map((item) => ({ ...item }));

  return dto;
}

export function toWireNotesQuery(query: NotesQuery): WireNotesQuery {
  return {
    ...query,
    tags: [...query.tags],
    languages: [...query.languages],
    now: toIsoString(query.now, 'now'),
  };
}

function toSection(dto: WireNoteSection): NoteSection {
  return { ...dto, notes: dto.notes.map(toNote) };
}

export function toNotesView(dto: WireNotesView): NotesView {
  return { ...dto, sections: dto.sections.map(toSection) };
}
