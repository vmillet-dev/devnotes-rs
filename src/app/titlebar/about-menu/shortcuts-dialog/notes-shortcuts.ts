import { ShortcutGroup } from '@core/services/shortcuts/shortcut.model';
import { CANVAS_SHORTCUT_GROUP } from '@notes/canvas-keys';

/**
 * The canvas group comes from `CanvasKeyboardDirective`, where the same table binds the
 * keys; the others are documentation only, handled by the editors and the palette — a Note's
 * are TipTap's own, and the Markdown it reads as it is typed.
 */
export const NOTES_SHORTCUT_GROUPS: readonly ShortcutGroup[] = [
  CANVAS_SHORTCUT_GROUP,
  {
    id: 'notes.editor',
    labelKey: 'shortcuts.groups.editor',
    shortcuts: [
      { keys: ['Escape'], labelKey: 'shortcuts.editor.close' },
      { keys: ['Enter'], labelKey: 'shortcuts.editor.newItem' },
      { keys: ['Backspace'], labelKey: 'shortcuts.editor.removeItem' },
      { keys: ['Alt', '↑ ↓'], labelKey: 'shortcuts.editor.moveItem' },
      { keys: ['Shift', 'Alt', 'F'], labelKey: 'shortcuts.editor.format' },
      { keys: ['Ctrl', 'F'], labelKey: 'shortcuts.editor.find' },
      { keys: ['Ctrl', 'H'], labelKey: 'shortcuts.editor.replace' },
      { keys: ['Ctrl', 'Alt', 'G'], labelKey: 'shortcuts.editor.goToLine' },
      { keys: ['Tab'], labelKey: 'shortcuts.editor.indent' },
      { keys: ['Shift', 'Tab'], labelKey: 'shortcuts.editor.outdent' },
    ],
  },
  {
    id: 'notes.note',
    labelKey: 'shortcuts.groups.note',
    shortcuts: [
      { keys: ['Ctrl', 'B'], labelKey: 'shortcuts.note.bold' },
      { keys: ['Ctrl', 'I'], labelKey: 'shortcuts.note.italic' },
      { keys: ['Ctrl', 'Shift', 'S'], labelKey: 'shortcuts.note.strike' },
      { keys: ['Ctrl', 'E'], labelKey: 'shortcuts.note.code' },
      { keys: ['Ctrl', 'K'], labelKey: 'shortcuts.note.link' },
      { keys: ['Ctrl'], labelKey: 'shortcuts.note.openLink' },
      { keys: ['Ctrl', 'Alt', '1 … 3'], labelKey: 'shortcuts.note.heading' },
      { keys: ['Ctrl', 'Shift', '8'], labelKey: 'shortcuts.note.bulletList' },
      { keys: ['Ctrl', 'Shift', '7'], labelKey: 'shortcuts.note.orderedList' },
      { keys: ['Ctrl', 'Shift', '9'], labelKey: 'shortcuts.note.taskList' },
      { keys: ['Ctrl', 'Shift', 'B'], labelKey: 'shortcuts.note.quote' },
      { keys: ['Ctrl', 'Alt', 'C'], labelKey: 'shortcuts.note.codeBlock' },
      { keys: ['#', '-', '1.', '[ ]', '>', '```'], labelKey: 'shortcuts.note.markdown' },
    ],
  },
  {
    id: 'notes.palette',
    labelKey: 'shortcuts.groups.palette',
    shortcuts: [
      { keys: ['@'], labelKey: 'shortcuts.palette.scope' },
      { keys: ['#'], labelKey: 'shortcuts.palette.tag' },
      { keys: ['>'], labelKey: 'shortcuts.palette.command' },
      { keys: ['Tab'], labelKey: 'shortcuts.palette.chip' },
      { keys: ['Backspace'], labelKey: 'shortcuts.palette.unchip' },
      { keys: ['↑ ↓'], labelKey: 'shortcuts.palette.navigate' },
      { keys: ['Enter'], labelKey: 'shortcuts.palette.open' },
      { keys: ['Ctrl', 'C'], labelKey: 'shortcuts.palette.paste' },
      { keys: ['Escape'], labelKey: 'shortcuts.palette.close' },
    ],
  },
];
