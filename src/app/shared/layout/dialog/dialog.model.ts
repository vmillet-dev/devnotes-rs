/**
 * ⚠️ The order is the value: a rung's position is both its `z-index` and its Escape priority,
 * and no stylesheet carries a modal `z-index`. The banners (`banners/`, at 80) stay above every
 * modal, which can trigger them. `app` — the help panels — sits under the editor, which a
 * global shortcut can open over them.
 */
const LAYERS = ['app', 'editor', 'settings', 'update', 'palette', 'fields', 'zoom', 'passphrase'] as const;

export type DialogLayer = (typeof LAYERS)[number];

const FIRST_RUNG = 50;

export function dialogRung(layer: DialogLayer): number {
  return FIRST_RUNG + LAYERS.indexOf(layer);
}

/**
 * - `fitted` — height follows the content, the panel is padded.
 * - `framed` — fixed height with a scrolling middle, so changing page does not make the
 *   panel jump under the cursor. Its sections carry their own padding.
 * - `bare` — no surface at all: what is shown *is* the content (an image).
 */
export type DialogVariant = 'fitted' | 'framed' | 'bare';
