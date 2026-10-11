import { ShortcutGroup } from '@core/services/shortcuts/shortcut.model';

/** Documentation only: the HTTP page and its rail bind these themselves. */
export const HTTP_SHORTCUT_GROUP: ShortcutGroup = {
  id: 'http',
  labelKey: 'shortcuts.groups.http',
  shortcuts: [
    { keys: ['Ctrl', 'Enter'], labelKey: 'shortcuts.http.send' },
    { keys: ['Ctrl', 'S'], labelKey: 'shortcuts.http.save' },
    { keys: ['Alt', '↑ ↓'], labelKey: 'shortcuts.http.moveRow' },
    { keys: ['Alt', '→'], labelKey: 'shortcuts.http.moveIn' },
    { keys: ['Alt', '←'], labelKey: 'shortcuts.http.moveOut' },
    { keys: ['F2'], labelKey: 'shortcuts.http.rename' },
    { keys: ['Delete'], labelKey: 'shortcuts.http.delete' },
  ],
};
