/**
 * The chapters of the written guide, in the order they are met.
 *
 * One list, here rather than in the dialog: the canvas and the board link straight into
 * a chapter now, and a second copy of these ids would let a link point at nothing.
 */
export const GUIDE_CHAPTERS = [
  'notes',
  'spaces',
  'folders',
  'organise',
  'fields',
  'checklists',
  'palette',
  'attachments',
  'trash',
  'transfer',
] as const;

export type GuideChapter = (typeof GUIDE_CHAPTERS)[number];
