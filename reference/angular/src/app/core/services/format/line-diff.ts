/** Past this many cells the table costs more than the answer is worth: every line counts. */
const MAX_CELLS = 4_000_000;

/**
 * The lines of `after` that are not in the longest run it shares with `before`, zero-based:
 * what a format touched, for the gutter and for the count it announces.
 */
export function changedLines(before: string, after: string): number[] {
  const old = before.split('\n');
  const next = after.split('\n');

  let start = 0;
  while (start < old.length && start < next.length && old[start] === next[start]) start++;
  let oldEnd = old.length;
  let nextEnd = next.length;
  while (oldEnd > start && nextEnd > start && old[oldEnd - 1] === next[nextEnd - 1]) {
    oldEnd--;
    nextEnd--;
  }

  const rows = oldEnd - start;
  const columns = nextEnd - start;
  if (rows === 0 || rows * columns > MAX_CELLS) return range(start, nextEnd);

  // Common-subsequence lengths from the end, so the walk below reads forwards.
  const width = columns + 1;
  const table = new Uint32Array((rows + 1) * width);
  for (let i = rows - 1; i >= 0; i--) {
    for (let j = columns - 1; j >= 0; j--) {
      table[i * width + j] =
        old[start + i] === next[start + j]
          ? table[(i + 1) * width + j + 1]! + 1
          : Math.max(table[(i + 1) * width + j]!, table[i * width + j + 1]!);
    }
  }

  const changed: number[] = [];
  let i = 0;
  let j = 0;
  while (j < columns) {
    if (i < rows && old[start + i] === next[start + j]) {
      i++;
      j++;
    } else if (i < rows && table[(i + 1) * width + j]! >= table[i * width + j + 1]!) {
      i++;
    } else {
      changed.push(start + j);
      j++;
    }
  }
  return changed;
}

function range(from: number, to: number): number[] {
  return Array.from({ length: to - from }, (_, index) => from + index);
}
