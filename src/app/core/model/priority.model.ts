import type { Priority } from './note.model';

/** In the order of their keys, 0 to 4: the menus list them so, and the canvas key reads its index. */
export const PRIORITIES: readonly Priority[] = ['none', 'low', 'medium', 'high', 'urgent'];

/** How many of the pill's three bars a level lights; the name tells the two top ones apart. */
export const PRIORITY_BARS: Readonly<Record<Priority, number>> = {
  none: 0,
  low: 1,
  medium: 2,
  high: 3,
  urgent: 3,
};
