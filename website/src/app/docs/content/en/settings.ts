import { downloadPath } from '../../../core/lang';
import { REPOSITORY } from '../../../core/site';
import { docPath } from '../../doc-index';
import { DocPage, b, code, field, h2, h3, keys, link, p, table, tip, ui, ul } from '../../doc.model';

export const TRANSFER: DocPage = {
  id: 'transfer',
  title: 'Import and export',
  summary:
    'Exporting everything, a space or a selection into a .devnotes archive, protected or not, importing it again without duplicates, and copying notes as Markdown.',
  body: [
    h2('export', 'Exporting'),
    p('The ', ui('File'), ' menu exports, as you choose:'),
    ul(
      [ui('Export everything…'), ': the whole library;'],
      [ui('Export active space…'), ': the open space;'],
      [ui('Export selection…'), ': the ticked notes.'],
    ),
    p(
      'The export is a ',
      code('.devnotes'),
      ' archive: the notes, their spaces and their attachments. Snippet versions are not part of it.',
    ),
    h3('protect', 'Protecting an export'),
    p(
      'Before writing the file, DevNotes asks ',
      ui('Protect this export?'),
      ' A passphrase seals it with a key of its own that travels with the file, not with this machine: whoever you give the phrase to can open it, no one else. ',
      ui('Export in the clear'),
      ' writes every note and every attachment readable by anyone who holds the file.',
    ),
    tip(
      'A plain export, kept somewhere safe, is the only copy that does not depend on your passphrase. See ',
      link('Security and backups', docPath('safety', 'en')),
      '.',
    ),
    h2('import', 'Importing'),
    p(
      ui('File → Import…'),
      ' reads a ',
      code('.devnotes'),
      ' archive (or an older ',
      code('.json'),
      ' export). A protected archive asks for its passphrase first.',
    ),
    ul(
      ['An import merges, it never replaces: spaces are matched by their name.'],
      [
        'A note already present is counted as skipped, not overwritten: importing the same file again creates no duplicate.',
      ],
      ['DevNotes always says what it imported, even when that is no note at all.'],
    ),
    h2('markdown', 'Copying as Markdown'),
    p(
      ui('File → Copy selection as Markdown'),
      ' puts the ticked notes on the clipboard: for each, its title, its space, its context, its tags, then its content in a code block. Enough to paste a procedure into a ticket or a wiki.',
    ),
  ],
};

export const SETTINGS: DocPage = {
  id: 'settings',
  title: 'Settings',
  summary:
    'Theme, language and density, the editor, how the window behaves, global variables, shortcuts and security.',
  body: [
    p(
      ui('File → Preferences…'),
      ' opens four pages. The theme and the density show while you choose; nothing is saved before ',
      ui('Apply'),
      ' or ',
      ui('OK'),
      '.',
    ),
    h2('general', 'General'),
    table(
      'The general settings',
      ['Group', 'Settings'],
      [
        b('Appearance'),
        'the language (French or English), the theme (System, Dark, Light), the density (Comfortable, Compact)',
      ],
      [b('Editor'), 'the default note type, indenting with Tab, line wrapping in code'],
      [b('Behaviour'), 'start with the system, minimise and close to the system tray'],
      [b('Quick paste'), 'pinned notes first in the palette'],
      [b('Notifications'), 'acknowledge each copy, announce updates'],
    ),
    p(
      'The ',
      ui('Compact'),
      ' density tightens the spacing, never the text. Without a system tray, the window minimises and closes normally: DevNotes refuses to hide where nothing could call it back.',
    ),
    h2('variables', 'Variables'),
    p(
      'The default values of your snippets’ fields, valid for every note: a ',
      code('host'),
      ' variable is offered to every ',
      field('host'),
      '. What is typed on a note comes first. A name is limited to letters, digits, hyphens and underscores.',
    ),
    h2('shortcuts', 'Shortcuts'),
    p(
      'The three global shortcuts, the canvas ones and the areas one. See ',
      link('Quick paste and shortcuts', docPath('paste', 'en')),
      '.',
    ),
    h2('security', 'Security'),
    ul(
      [b('Passphrase'), ': change it, for the next launch.'],
      [b('Backup copies'), ': copy the library at launch, and restore a copy.'],
    ),
    p('Both are detailed in ', link('Security and backups', docPath('safety', 'en')), '.'),
    h2('per-library', 'What each library remembers'),
    p(
      'The preferences above apply to the application. Each library also remembers its own display choices, such as the ',
      ui('List'),
      ' or ',
      ui('Board'),
      ' view, the sort, or the API client’s active environment.',
    ),
  ],
};

export const FAQ: DocPage = {
  id: 'faq',
  title: 'Questions',
  summary:
    'The price, macOS, syncing, a forgotten passphrase, the Windows warning, and where your data goes.',
  body: [
    h2('free', 'Is DevNotes free?'),
    p(
      'Yes, and open source: it is published under the GPL-3.0 licence, its code is on ',
      link('GitHub', REPOSITORY),
      '. No account, no subscription, no advertising.',
    ),
    h2('macos', 'Is there a macOS version?'),
    p(
      'Not for now. It cannot be tested here, and Gatekeeper requires a paid Apple Developer account, with no free way round it on recent versions.',
    ),
    h2('sync', 'Can I sync two computers?'),
    p(
      'DevNotes syncs nothing: that is what guarantees your notes do not leave your machine. To move notes from one computer to another, export them into a protected archive, then import it on the other side. An import never creates a duplicate.',
    ),
    h2('forgotten', 'I have forgotten my passphrase.'),
    p(
      'It cannot be recovered: it wraps the key that seals your notes, and nothing else opens it. ',
      ui('Forgotten your passphrase?'),
      ' archives the library, still sealed, and starts a new one. A plain export kept apart is the only copy that does not depend on it.',
    ),
    h2('smartscreen', 'Windows shows a warning on first launch.'),
    p(
      'Choose ',
      ui('More info'),
      ', then ',
      ui('Run anyway'),
      '. The warning is about the publisher’s identity: DevNotes has no code signing certificate. Each version publishes ',
      code('SHA256SUMS.txt'),
      ' to check the files, and updates are signed and verified before they are installed.',
    ),
    h2('offline', 'Does DevNotes work offline?'),
    p(
      'Entirely. Only the API client, which contacts the servers you give it, and the update check need the network.',
    ),
    h2('updates', 'How do I update?'),
    p(
      'DevNotes offers new versions itself, with their notes. You can also download ',
      link('the latest version', downloadPath('en')),
      ' and install it over the old one: your libraries are not touched.',
    ),
    h2('data', 'Where are my notes?'),
    p(
      'In your profile folder: ',
      code('%APPDATA%\\com.devnotes.app\\'),
      ' on Windows, ',
      code('~/.local/share/com.devnotes.app/'),
      ' on Linux. The details are in ',
      link('Security and backups', docPath('safety', 'en')),
      '.',
    ),
    h2('report', 'I found a bug, or I have an idea.'),
    p(
      'Open an issue on ',
      link('the GitHub repository', `${REPOSITORY}/issues`),
      ', saying what you were doing, what you expected and what happened. The version is in ',
      ui('About'),
      '.',
    ),
    tip('Nothing you type in DevNotes is sent with an issue: copy only what helps to understand.'),
    h2('shortcut-taken', 'Ctrl Alt P does nothing.'),
    p(
      'Another application probably already holds that combination: DevNotes says so at startup. Choose another in ',
      ui('Preferences… → Shortcuts'),
      ', or free it in the other application. Global shortcuts work with the window closed as long as DevNotes runs in the system tray; once ',
      ui('Quit DevNotes'),
      ' is chosen, ',
      keys('Ctrl Alt P'),
      ' has no application left to wake.',
    ),
  ],
};
