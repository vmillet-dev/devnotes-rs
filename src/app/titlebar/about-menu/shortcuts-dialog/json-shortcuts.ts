import { ShortcutGroup } from '@core/services/shortcuts/shortcut.model';

/** Documentation only: the explorer, its graph and its tree bind these themselves. */
export const JSON_SHORTCUT_GROUP: ShortcutGroup = {
  id: 'json',
  labelKey: 'shortcuts.groups.json',
  shortcuts: [
    { keys: ['↑ ↓ ← →'], labelKey: 'shortcuts.json.navigate' },
    { keys: ['Enter'], labelKey: 'shortcuts.json.toggle' },
    { keys: ['Shift', '→'], labelKey: 'shortcuts.json.openAll' },
    { keys: ['Shift', '←'], labelKey: 'shortcuts.json.collapseAll' },
    { keys: ['Alt', '↑ ↓ ← →'], labelKey: 'shortcuts.json.moveNode' },
    { keys: ['Enter'], labelKey: 'shortcuts.json.nextMatch' },
    { keys: ['Shift', 'Enter'], labelKey: 'shortcuts.json.previousMatch' },
  ],
};
