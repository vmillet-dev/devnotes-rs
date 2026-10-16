import { Rebindable, ShortcutGroup, acceleratorKeys } from '@core/services/shortcuts/shortcut.model';

/** In the switch's order, which is also the order of their keys. */
export const AREAS = ['notes', 'tools', 'http'] as const;
export type Area = (typeof AREAS)[number];

/** Read by position, not by character: on AZERTY, the digit row types `&é"` unshifted. */
export function areaForKey(event: KeyboardEvent): Area | null {
  if (!(event.ctrlKey || event.metaKey) || event.altKey || event.shiftKey) return null;

  const rank = /^(?:Digit|Numpad)([1-9])$/.exec(event.code)?.[1];
  return rank === undefined ? null : (AREAS[Number(rank) - 1] ?? null);
}

/** Ctrl+Shift+T, the tools' search from anywhere: a letter, read as printed like the canvas keys. */
export function isToolSearchKey(event: KeyboardEvent): boolean {
  return (
    (event.ctrlKey || event.metaKey) && event.shiftKey && !event.altKey && event.key.toLowerCase() === 't'
  );
}

/** Ctrl+K, the search of the area on screen: the notes' field, or the tools'. */
export function isSearchKey(event: KeyboardEvent): boolean {
  return (
    (event.ctrlKey || event.metaKey) && !event.shiftKey && !event.altKey && event.key.toLowerCase() === 'k'
  );
}

/**
 * Hides and shows the rail of whichever area is on screen. It was the canvas's key, and HTTP
 * had a copy of it; the tools had none.
 */
export const RAIL_ACTION: Rebindable = { id: 'areas.rail', labelKey: 'areas.rail', fallback: 'Ctrl+B' };

/** What the preferences let be moved among the keys that act in every area. */
export const AREA_ACTIONS: readonly Rebindable[] = [RAIL_ACTION];

export const AREA_SHORTCUT_GROUP: ShortcutGroup = {
  id: 'areas',
  labelKey: 'shortcuts.groups.areas',
  shortcuts: [
    ...AREAS.map((area, index) => ({ keys: ['Ctrl', String(index + 1)], labelKey: `areas.go.${area}` })),
    { keys: ['Ctrl', 'K'], labelKey: 'shortcuts.canvas.search' },
    { keys: ['Ctrl', 'Shift', 'T'], labelKey: 'tools.searchShortcut' },
    { keys: acceleratorKeys(RAIL_ACTION.fallback), labelKey: RAIL_ACTION.labelKey, action: RAIL_ACTION },
  ],
};
