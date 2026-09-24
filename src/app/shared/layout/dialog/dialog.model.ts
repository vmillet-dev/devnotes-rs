/**
 * ⚠️ The order is the value: a rung's position is both its `z-index` and its Escape priority,
 * and no stylesheet carries a modal `z-index`. The banners (`banners/`, at 80) stay above every
 * modal, which can trigger them. `titlebar` is no dialog but the titlebar's menus, which a
 * full-screen editor leaves in reach; `app`, the help panels they open, lands over the editor.
 */
const LAYERS = [
  'editor',
  'titlebar',
  'app',
  'settings',
  'update',
  'palette',
  'fields',
  'zoom',
  'passphrase',
] as const;

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
