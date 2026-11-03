const OPEN = '{{';
const CLOSE = '}}';

/** No digit and no `x`, which the stand-ins are made of besides the marker. */
const MARKER_LETTERS = 'qzjkvw';

interface Swap {
  readonly original: string;
  readonly standIn: string;
}

export interface Shielded {
  readonly text: string;
  readonly cursor: number;
  readonly marker: string;
  readonly swaps: readonly Swap[];
}

/**
 * Every `{{…}}` becomes an identifier of its own length, which every language Prettier formats
 * accepts where a value goes: read as code, `{{db_host}}` is two nested blocks in JavaScript
 * and a flow mapping in YAML. A field or not — Angular's `{{ user.name }}` would be reflowed too.
 * `null` when no marker is free, which only a text written against this function could cause.
 */
export function shieldFields(text: string, cursor: number): Shielded | null {
  const marker = freeMarker(text);
  if (marker === null) return null;

  const swaps: Swap[] = [];
  let shielded = '';
  let shieldedCursor = cursor;
  let rest = 0;

  for (;;) {
    const start = text.indexOf(OPEN, rest);
    const end = start < 0 ? -1 : text.indexOf(CLOSE, start + OPEN.length);
    if (end < 0) break;

    const original = text.slice(start, end + CLOSE.length);
    const standIn = standInFor(marker, swaps.length, original.length);
    shielded += text.slice(rest, start);
    if (cursor > start) {
      shieldedCursor =
        cursor >= start + original.length
          ? shieldedCursor + standIn.length - original.length
          : shielded.length;
    }
    shielded += standIn;
    swaps.push({ original, standIn });
    rest = start + original.length;
  }

  return { text: shielded + text.slice(rest), cursor: shieldedCursor, marker, swaps };
}

/**
 * The fields back, in order. `null` when one is missing, moved past another or copied: the
 * text is then not the one that was typed, and nothing of it may be applied.
 */
export function unshieldFields(
  formatted: string,
  cursor: number,
  shielded: Shielded,
): { text: string; cursor: number } | null {
  let text = '';
  let restoredCursor = cursor;
  let from = 0;

  for (const { original, standIn } of shielded.swaps) {
    const at = formatted.indexOf(standIn, from);
    if (at < 0) return null;

    text += formatted.slice(from, at) + original;
    if (cursor >= at + standIn.length) restoredCursor += original.length - standIn.length;
    from = at + standIn.length;
  }
  text += formatted.slice(from);

  return text.includes(shielded.marker) ? null : { text, cursor: restoredCursor };
}

function freeMarker(text: string): string | null {
  for (const first of MARKER_LETTERS) {
    for (const second of MARKER_LETTERS) {
      if (first !== second && !text.includes(first + second)) return first + second;
    }
  }
  return null;
}

/** The marker on both sides, so no stand-in is found inside another. */
function standInFor(marker: string, index: number, length: number): string {
  const core = `${marker}${index}`;
  return core + 'x'.repeat(Math.max(0, length - core.length - marker.length)) + marker;
}
