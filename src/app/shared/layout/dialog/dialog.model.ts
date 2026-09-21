/**
 * ⚠️ The order is the value: a rung's position decides both the `z-index` and which
 * dialog Escape reaches, and no stylesheet carries a modal `z-index`. The banners of
 * `layout/` sit at 80 and must stay above every modal — they are triggered from inside
 * one — hence the base well below it.
 *
 * ⚠️ `app` is the About menu's four help panels, and it sits **under** the editor. A
 * help panel covers the whole page, so nothing on the canvas can be reached while one is
 * up — except through a global shortcut, which comes from outside the application
 * altogether. Opening a note from the palette then drew it behind the help.
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
