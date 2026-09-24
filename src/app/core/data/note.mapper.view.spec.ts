import { describe, expect, it } from 'vitest';
import type { NotesView as WireNotesView } from '@core/ipc/bindings';
import { NotesQuery } from '@core/model/note.model';
import { toNotesView, toWireNotesQuery } from './note.mapper';

const BASE_VIEW: WireNotesView = {
  sections: [],
  availableTags: ['api'],
  availableLanguages: ['json', 'yml'],
  isFiltering: false,
  matched: 0,
};

const BASE_QUERY: NotesQuery = {
  spaceId: 'space-1',
  folderId: null,
  search: 'deploy',
  filter: 'all',
  tags: ['api'],
  languages: ['json'],
  now: new Date('2026-01-01T10:00:00.000Z'),
  tzOffsetMinutes: -120,
  pinnedFirst: true,
};

describe('toWireNotesQuery', () => {
  it('sends the selected languages across the bridge', () => {
    expect(toWireNotesQuery(BASE_QUERY).languages).toEqual(['json']);
  });

  it('carries the pinned-first flag, which the palette alone turns off', () => {
    expect(toWireNotesQuery({ ...BASE_QUERY, pinnedFirst: false }).pinnedFirst).toBe(false);
  });

  it('sends an empty list rather than omitting the field', () => {
    expect(toWireNotesQuery({ ...BASE_QUERY, languages: [] }).languages).toEqual([]);
  });
});

describe('toNotesView', () => {
  it('reads the languages offered for the rail', () => {
    expect(toNotesView(BASE_VIEW).availableLanguages).toEqual(['json', 'yml']);
  });
});
