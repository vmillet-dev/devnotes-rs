import { ShortcutGroup } from '@core/services/shortcuts/shortcut.model';

/** In the switch's order, which is also the order of their keys. HTTP joins with its client. */
export const AREAS = ['notes', 'tools'] as const;
export type Area = (typeof AREAS)[number];

/** Read by position, not by character: on AZERTY, the digit row types `&é"` unshifted. */
export function areaForKey(event: KeyboardEvent): Area | null {
  if (!(event.ctrlKey || event.metaKey) || event.altKey || event.shiftKey) return null;

  const rank = /^(?:Digit|Numpad)([1-9])$/.exec(event.code)?.[1];
  return rank === undefined ? null : (AREAS[Number(rank) - 1] ?? null);
}

export const AREA_SHORTCUT_GROUP: ShortcutGroup = {
  id: 'areas',
  labelKey: 'shortcuts.groups.areas',
  shortcuts: AREAS.map((area, index) => ({
    keys: ['Ctrl', String(index + 1)],
    labelKey: `areas.go.${area}`,
  })),
};
