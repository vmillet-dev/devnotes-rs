import { docPath } from '../../doc-index';
import { DocPage, b, code, h2, h3, keys, link, ol, p, table, tip, ui, ul, warn } from '../../doc.model';

export const SAFETY: DocPage = {
  id: 'safety',
  title: 'Security and backups',
  summary:
    'What is encrypted and how, the passphrase, several libraries, backup copies, the trash, versions and undo.',
  body: [
    h2('what-leaves', 'What leaves your machine'),
    p(
      'Nothing, except what you send. No account, no cloud, no telemetry. A request from the API client goes to the server you name, and nowhere else; the tools call nothing; the update check only asks GitHub whether a newer version exists.',
    ),
    h2('encryption', 'Encryption'),
    p(
      'DevNotes asks for your passphrase once at each launch. It opens the library and is stored nowhere: no keychain, no “remember me”.',
    ),
    table(
      'How a library is sealed',
      ['What', 'How'],
      [
        b('Sealed on disk'),
        'titles, bodies, sources and their history, list items, names of spaces and folders, field values, names and contents of attachments',
      ],
      [b('Key derivation'), 'Argon2id (64 MiB, 3 passes) over a random salt'],
      [b('Encryption'), 'AES-256-GCM, a fresh nonce on every write; a tampered value is refused'],
      [b('Passphrase'), 'at least twelve characters; it wraps the library’s key, it is not the key'],
    ),
    h3('what-stays-readable', 'What a stolen file shows'),
    p(
      'What the database filters and sorts on is not sealed: sealing it would mean decrypting everything to answer a search. Without the passphrase, no one reads a title, a body or an attachment, but they see:',
    ),
    ul(
      ['the number of notes and how they are spread across spaces and folders, without their names;'],
      ['the dates of creation, modification and due dates, and the pinned notes;'],
      ['the language and kind of each note;'],
      ['every tag and every field name, as you typed them;'],
      ['the length of each sealed value.'],
    ),
    tip(
      'So do not let a tag or a field name carry the secret: ',
      code('db_password'),
      ' stays readable, its value does not.',
    ),
    h2('passphrase', 'The passphrase'),
    p(
      'It is changed in ',
      ui('Preferences… → Security → Change…'),
      ': the new one applies at the next launch. Nothing is re-encrypted, the phrase only wraps the key; the backup copies kept beside it are rewrapped with it.',
    ),
    warn(
      b('There is no recovery.'),
      ' A forgotten phrase cannot be reset. ',
      ui('Forgotten your passphrase?'),
      ' only offers to archive the library, still sealed, and to start a new one. Keep a plain export somewhere safe if your notes matter.',
    ),
    h2('libraries', 'Several libraries'),
    p(
      ui('File → Libraries…'),
      ' creates, renames, opens or deletes them. Each has its own database, attachments, backup copies and passphrase: nothing passes from one to another. Switching library reloads the window; neither the open one nor the last one can be deleted.',
    ),
    h2('backups', 'Backup copies'),
    p(
      'With ',
      ui('Copy the library at launch'),
      ' (on by default), DevNotes takes a copy at startup, one a day at most, and keeps the last three: the database, the key that opens it and the attachments. Attachments are linked rather than duplicated: a copy costs barely more than the notes.',
    ),
    p(
      ui('Preferences… → Security → Restore…'),
      ' replaces the whole library with a copy. Nothing is deleted: the current one is set aside, and you are told where.',
    ),
    tip(
      'These copies are kept beside the original: they cover a trash emptied too soon or a failed update, not a dead disk. For that, ',
      link('export', docPath('transfer', 'en')),
      ' elsewhere.',
    ),
    h2('trash', 'The trash and Ctrl Z'),
    p(
      'A deleted note goes to the trash and stays there thirty days. The banner that follows offers ',
      ui('Undo'),
      ', and ',
      keys('Ctrl Z'),
      ' still does it after the banner has gone. The ',
      ui('Trash'),
      ', at the bottom of the sidebar, restores a note or deletes it for good.',
    ),
    h2('versions', 'A snippet’s versions'),
    p(
      'A snippet keeps its last twenty versions. The editor’s ',
      ui('History'),
      ' shows what would come back and what would go before you ',
      ui('Go back to this version'),
      '. Versions are not included in exports.',
    ),
    h2('integrity', 'If the database is damaged'),
    p(
      'The database is checked at every opening. If it is damaged, DevNotes says so instead of starting on it, and offers to set it aside with what can still be recovered. Your passphrase does not change.',
    ),
    h2('your-files', 'Where your files are'),
    table(
      'The folder of your libraries',
      ['System', 'Folder'],
      ['Windows', code('%APPDATA%\\com.devnotes.app\\')],
      ['Linux', code('~/.local/share/com.devnotes.app/')],
    ),
    p(
      'Each library lives in ',
      code('libraries/<id>/'),
      ': the database, ',
      code('vault.json'),
      ' (the key your phrase opens), the attachments and the backup copies.',
    ),
    warn(
      'The database and ',
      code('vault.json'),
      ' are only worth something together. Copy one without the other, and you will have copied something no one can open again.',
    ),
    h2('in-memory', 'While DevNotes runs'),
    ol(
      ['The key stays in memory for the session: there is no relocking after inactivity.'],
      ['Opening an attachment writes a decrypted copy to your profile, erased when DevNotes closes.'],
      [
        'DevNotes protects your files at rest (a stolen laptop, a copied profile), not a machine already compromised while it runs.',
      ],
    ),
  ],
};
