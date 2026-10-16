import { NoteKind } from '@core/model/note.model';
import { AREAS, Area, areaKeys } from '@core/services/areas/area.model';
import { Rebindable } from '@core/services/shortcuts/shortcut.model';
import { CREATE_ACTIONS } from '@notes/canvas-keys';

/**
 * What `>` lists. A command names its key from the table that binds it — the canvas's for a
 * new note, the areas' for an area — so the palette never shows a key the application lost.
 */
export type PaletteCommand =
  | {
      readonly kind: 'create';
      readonly note: NoteKind;
      readonly labelKey: string;
      readonly action: Rebindable;
    }
  | { readonly kind: 'request'; readonly labelKey: string }
  | { readonly kind: 'folder'; readonly labelKey: string }
  | {
      readonly kind: 'area';
      readonly area: Area;
      readonly labelKey: string;
      readonly keys: readonly string[];
    }
  | { readonly kind: 'preferences'; readonly labelKey: string }
  | { readonly kind: 'shortcuts'; readonly labelKey: string };

export const PALETTE_COMMANDS: readonly PaletteCommand[] = [
  ...(Object.keys(CREATE_ACTIONS) as NoteKind[]).map((note): PaletteCommand => ({
    kind: 'create',
    note,
    labelKey: CREATE_ACTIONS[note].labelKey,
    action: CREATE_ACTIONS[note],
  })),
  { kind: 'request', labelKey: 'palette.commands.newRequest' },
  { kind: 'folder', labelKey: 'palette.commands.newFolder' },
  ...AREAS.map((area): PaletteCommand => ({
    kind: 'area',
    area,
    labelKey: `areas.go.${area}`,
    keys: areaKeys(area),
  })),
  { kind: 'preferences', labelKey: 'palette.commands.preferences' },
  { kind: 'shortcuts', labelKey: 'palette.commands.shortcuts' },
];

/** Its place in the table: what a row is told apart by, in the DOM as in a `track`. */
export function commandId(command: PaletteCommand): string {
  switch (command.kind) {
    case 'create':
      return `create-${command.note}`;
    case 'area':
      return `area-${command.area}`;
    case 'request':
    case 'folder':
    case 'preferences':
    case 'shortcuts':
      return command.kind;
  }
}
