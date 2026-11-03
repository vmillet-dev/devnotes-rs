/**
 * The column count is only known on screen, so positions are measured: rows are deduced
 * from equal `top`s, and the nearest column is picked on `left`.
 */
export interface CardBox {
  readonly top: number;
  readonly left: number;
}

export type FocusDirection = 'prev' | 'next' | 'up' | 'down';

/** Two cards of the same row can differ by a few pixels. */
const ROW_TOLERANCE = 4;

interface PositionedCard {
  readonly index: number;
  readonly box: CardBox;
}

interface Row {
  readonly top: number;
  readonly cards: PositionedCard[];
}

function rowsOf(boxes: readonly CardBox[]): readonly Row[] {
  const rows: Row[] = [];

  boxes.forEach((box, index) => {
    const row = rows.find((candidate) => Math.abs(candidate.top - box.top) <= ROW_TOLERANCE);
    if (row) {
      row.cards.push({ index, box });
    } else {
      rows.push({ top: box.top, cards: [{ index, box }] });
    }
  });

  return rows;
}

/** `current` when the move leaves the grid: stopping beats wrapping around. */
export function nextFocusIndex(
  boxes: readonly CardBox[],
  current: number,
  direction: FocusDirection,
): number {
  if (boxes.length === 0) return -1;

  const currentBox = boxes[current];
  if (!currentBox) return 0;

  if (direction === 'prev') return Math.max(0, current - 1);
  if (direction === 'next') return Math.min(boxes.length - 1, current + 1);

  const rows = rowsOf(boxes);
  const rowIndex = rows.findIndex((row) => row.cards.some((card) => card.index === current));
  const targetRow = rows[rowIndex + (direction === 'down' ? 1 : -1)];
  if (!targetRow) return current;

  // A row exists because a card is in it, so `first` is always there.
  const [first, ...others] = targetRow.cards;
  if (!first) return current;

  const nearest = others.reduce(
    (closest, candidate) =>
      Math.abs(candidate.box.left - currentBox.left) < Math.abs(closest.box.left - currentBox.left)
        ? candidate
        : closest,
    first,
  );

  return nearest.index;
}
