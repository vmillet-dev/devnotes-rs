import { describe, expect, it } from 'vitest';
import { RequestHit } from '@core/model/http.model';
import { PALETTE_KINDS, PaletteKind } from '@core/state/palette.store';
import { createNote } from '@testing/note.fixture';
import { FAKE_TOOLS } from '@testing/tool-catalogue.fixture';
import { PaletteFound, recentGroups, searchGroups } from './palette-options';

const request = (id: string): RequestHit => ({
  id,
  name: id,
  kind: 'http',
  method: 'GET',
  url: '/',
  place: ['API'],
});

const FOUND: PaletteFound = {
  notes: Array.from({ length: 9 }, (_, index) => createNote({ id: `n${index}` })),
  requests: Array.from({ length: 5 }, (_, index) => request(`r${index}`)),
  tools: FAKE_TOOLS,
  spaces: [{ id: 's1', name: 'Travail', pinned: false }],
};

const every = new Set<PaletteKind>(PALETTE_KINDS);

describe('searchGroups', () => {
  it('shares the screen out between the kinds, each under its heading and count', () => {
    const groups = searchGroups(FOUND, every, 'x');

    expect(groups.map((group) => [group.heading, group.count])).toEqual([
      ['notes', 5],
      ['requests', 3],
      ['tools', 3],
      ['spaces', 1],
      [null, null],
    ]);
    expect(groups.at(-1)?.options).toEqual([{ kind: 'create', key: 'create', title: 'x' }]);
  });

  it('gives one kind alone the room, with no heading', () => {
    const groups = searchGroups(FOUND, new Set<PaletteKind>(['requests']), 'x');

    expect(groups).toHaveLength(1);
    expect(groups[0]?.heading).toBeNull();
    expect(groups[0]?.options).toHaveLength(5);
  });

  it('offers to create a note only while notes are searched and something is typed', () => {
    const tools = new Set<PaletteKind>(['tools']);

    expect(searchGroups(FOUND, tools, 'x').some((group) => group.options[0]?.kind === 'create')).toBe(false);
    expect(searchGroups(FOUND, every, '').some((group) => group.options[0]?.kind === 'create')).toBe(false);
  });

  it('draws no heading over a single kind that answered among several asked', () => {
    const groups = searchGroups({ ...FOUND, requests: [], tools: [], spaces: [] }, every, '');

    expect(groups.map((group) => group.heading)).toEqual([null]);
  });
});

describe('recentGroups', () => {
  it('lists a few of each kind under one heading, and no space', () => {
    const [group, ...rest] = recentGroups(FOUND);

    expect(rest).toEqual([]);
    expect(group?.heading).toBe('recent');
    expect(group?.options.map((option) => option.kind)).toEqual([
      'note',
      'note',
      'note',
      'note',
      'request',
      'request',
      'tool',
      'tool',
    ]);
  });

  it('is nothing at all on an empty library', () => {
    expect(recentGroups({ notes: [], requests: [], tools: [], spaces: [] })).toEqual([]);
  });
});
