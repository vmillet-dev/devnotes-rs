import { RequestHit } from '@core/model/http.model';
import { Note } from '@core/model/note.model';
import { Space } from '@core/model/space.model';
import { ToolDefinition } from '@core/services/tools/tool.model';
import { PALETTE_KINDS, PaletteChip, PaletteKind } from '@core/state/palette.store';
import { PaletteCommand, commandId } from './palette-commands';

export type PaletteOption =
  | { readonly kind: 'note'; readonly key: string; readonly note: Note }
  | { readonly kind: 'request'; readonly key: string; readonly request: RequestHit }
  | { readonly kind: 'tool'; readonly key: string; readonly tool: ToolDefinition }
  | { readonly kind: 'space'; readonly key: string; readonly space: Space }
  | { readonly kind: 'command'; readonly key: string; readonly command: PaletteCommand }
  /** A chip to add, offered after `@` or `#`: chosen, it narrows the search. */
  | { readonly kind: 'chip'; readonly key: string; readonly chip: PaletteChip; readonly count: number | null }
  | { readonly kind: 'create'; readonly key: string; readonly title: string };

export type PaletteHeading = PaletteKind | 'recent' | 'scopes' | 'spaceNotes' | 'tags' | 'commands';

export interface PaletteGroup {
  readonly heading: PaletteHeading | null;
  /** Beside the heading when the list holds more than one kind. */
  readonly count: number | null;
  readonly options: readonly PaletteOption[];
}

/** What each source answered, before the palette shares the screen out. */
export interface PaletteFound {
  readonly notes: readonly Note[];
  readonly requests: readonly RequestHit[];
  readonly tools: readonly ToolDefinition[];
  readonly spaces: readonly Space[];
}

/** Several kinds share the screen and each gives way; one alone has all of it. */
const SHARED: Readonly<Record<PaletteKind, number>> = { notes: 5, requests: 3, tools: 3, spaces: 3 };
const ALONE = 8;
/** Recent things, a few of each kind: the empty field is a way back to what was in hand. */
const RECENT: Readonly<Record<PaletteKind, number>> = { notes: 4, requests: 2, tools: 2, spaces: 0 };

function optionsOf(found: PaletteFound, kind: PaletteKind): readonly PaletteOption[] {
  switch (kind) {
    case 'notes':
      return found.notes.map((note) => ({ kind: 'note', key: `note-${note.id}`, note }));
    case 'requests':
      return found.requests.map((request) => ({ kind: 'request', key: `request-${request.id}`, request }));
    case 'tools':
      return found.tools.map((tool) => ({ kind: 'tool', key: `tool-${tool.id}`, tool }));
    case 'spaces':
      return found.spaces.map((space) => ({ kind: 'space', key: `space-${space.id}`, space }));
  }
}

/** By kind, in a fixed order; a heading and its count only when there is more than one. */
export function searchGroups(
  found: PaletteFound,
  searched: ReadonlySet<PaletteKind>,
  term: string,
): readonly PaletteGroup[] {
  const alone = searched.size === 1;
  const present = PALETTE_KINDS.filter((kind) => searched.has(kind) && optionsOf(found, kind).length > 0);
  const several = present.length > 1;

  const groups: PaletteGroup[] = present.map((kind) => {
    const options = optionsOf(found, kind).slice(0, alone ? ALONE : SHARED[kind]);
    return { heading: several ? kind : null, count: several ? options.length : null, options };
  });

  // After the results: finding something keeps first place.
  if (term !== '' && searched.has('notes')) {
    groups.push({ heading: null, count: null, options: [{ kind: 'create', key: 'create', title: term }] });
  }
  return groups;
}

export function recentGroups(found: PaletteFound): readonly PaletteGroup[] {
  const options = PALETTE_KINDS.flatMap((kind) => optionsOf(found, kind).slice(0, RECENT[kind]));
  return options.length > 0 ? [{ heading: 'recent', count: null, options }] : [];
}

export function chipGroup(
  heading: PaletteHeading,
  chips: readonly (readonly [PaletteChip, number | null])[],
): PaletteGroup {
  return {
    heading,
    count: null,
    options: chips.map(([chip, count]) => ({ kind: 'chip', key: `chip-${chipId(chip)}`, chip, count })),
  };
}

export function commandGroup(commands: readonly PaletteCommand[]): PaletteGroup {
  return {
    heading: 'commands',
    count: commands.length,
    options: commands.map((command) => ({ kind: 'command', key: `command-${commandId(command)}`, command })),
  };
}

export function chipId(chip: PaletteChip): string {
  switch (chip.kind) {
    case 'scope':
      return `scope-${chip.scope}`;
    case 'space':
      return `space-${chip.id}`;
    case 'tag':
      return `tag-${chip.tag}`;
  }
}
