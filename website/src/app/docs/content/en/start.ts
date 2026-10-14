import { downloadPath } from '../../../core/lang';
import { docPath } from '../../doc-index';
import { DocPage, b, code, h2, keys, link, ol, p, snippet, table, tip, ui, ul, warn } from '../../doc.model';

export const OVERVIEW: DocPage = {
  id: 'overview',
  title: 'Documentation',
  summary:
    'How to use DevNotes day to day: writing and organising your notes, pasting a snippet from anywhere, the tools, the API client and the safety of your data.',
  body: [
    p(
      'DevNotes keeps your snippets, notes and lists in an encrypted library on your machine, with some thirty offline tools and an API client. These pages explain how to use it, in the order you discover it. If you are new, start with ',
      link('getting started', docPath('start', 'en')),
      '.',
    ),
    { kind: 'guides' },
    h2('in-the-application', 'In the application'),
    p(
      'The title bar’s ',
      ui('About'),
      ' menu opens a ',
      ui('Getting started'),
      ' guide, the list of ',
      ui('Keyboard shortcuts'),
      ' active wherever you are, and the ',
      ui('What’s new'),
      ' of each version. Shortcuts are set in ',
      ui('File → Preferences… → Shortcuts'),
      '.',
    ),
  ],
};

export const GETTING_STARTED: DocPage = {
  id: 'start',
  title: 'Getting started',
  summary:
    'Installing DevNotes, choosing your library’s passphrase, and a tour of the sample notes of the first launch.',
  body: [
    h2('install', 'Installing'),
    p(
      'DevNotes runs on Windows and Linux. Every file is on ',
      link('the download page', downloadPath('en')),
      ', each as a direct link: take the one that matches your system.',
    ),
    table(
      'The files of each version',
      ['System', 'File', 'Take it if'],
      ['Windows', code('…_x64-setup.exe'), 'you simply want to install it: this is the one'],
      ['Windows', code('…_x64_en-US.msi'), 'you deploy through Group Policy'],
      ['Windows', code('…-windows.exe'), 'you want nothing installed'],
      ['Linux', code('…_amd64.AppImage'), 'any distribution, nothing to install'],
      ['Linux', code('…_amd64.deb'), 'Debian, Ubuntu and derivatives'],
      ['Linux', code('…x86_64.rpm'), 'Fedora, RHEL and derivatives'],
      ['Linux', code('…-linux'), 'the bare executable'],
    ),
    p(
      'The Windows installer installs for your account, without administrator rights. On Linux, an AppImage is made executable before it runs, and a package installs with your distribution’s package manager:',
    ),
    snippet(
      'Installing on Linux',
      'sh',
      'chmod +x devnotes_*_amd64.AppImage        # then run it\nsudo apt install ./devnotes_*_amd64.deb   # Debian and Ubuntu\nsudo dnf install ./devnotes-*.x86_64.rpm  # Fedora and RHEL',
    ),
    tip(
      'Windows shows a SmartScreen warning on first launch: choose ',
      ui('More info'),
      ', then ',
      ui('Run anyway'),
      '. It is about the publisher’s identity, not the file: DevNotes has no code signing certificate. Its updates, on the other hand, are signed and verified before they are installed.',
    ),
    h2('passphrase', 'Choosing a passphrase'),
    p(
      'On first launch, DevNotes asks you to ',
      ui('Protect this library'),
      ': choose a passphrase of at least twelve characters, and type it twice. It is asked once at each launch, and stored nowhere.',
    ),
    p(
      'Several unrelated words hold far better than one clever word followed by a digit: ',
      code('lamp bicycle storm peony'),
      ' is longer and easier to remember than ',
      code('P@ssw0rd2026'),
      '.',
    ),
    warn(
      b('There is no recovery.'),
      ' No account, no reset: a lost passphrase is a lost library. A plain export is the only copy that does not depend on it. See ',
      link('Security and backups', docPath('safety', 'en')),
      '.',
    ),
    h2('first-tour', 'The first tour'),
    p(
      'A new library holds a ',
      ui('Getting started'),
      ' space with a few sample notes: a welcome note, a ',
      ui('psql connection'),
      ' snippet with fields to fill, an Angular code note and a ',
      ui('Things to try'),
      ' list that walks you round:',
    ),
    ol(
      ['Open a note by clicking its card.'],
      ['Copy a note without opening it, from the card’s button.'],
      ['Copy ', ui('psql connection'), ' and fill its fields.'],
      ['Open the quick paste palette with ', keys('Ctrl Alt P'), '.'],
      ['Create a space of your own and move a note into it.'],
      ['Switch to the ', ui('Board'), ' view and drag a note into a zone.'],
    ),
    p('Tick it off from its card as you go. The sample notes are deleted like any other once read.'),
    h2('three-areas', 'Three areas'),
    p('The title bar switches from one area to another:'),
    ul(
      [b('Notes'), ' (', keys('Ctrl 1'), '): your snippets, notes and lists, in their spaces and folders.'],
      [b('Tools'), ' (', keys('Ctrl 2'), '): some thirty offline utilities.'],
      [b('HTTP'), ' (', keys('Ctrl 3'), '): the API client, its collections and environments.'],
    ),
    h2('updates', 'Updates'),
    p(
      'DevNotes checks by itself whether a newer version exists and offers it, with its release notes: ',
      ui('Install now'),
      ', ',
      ui('Later'),
      ' or ',
      ui('Do not offer version … again'),
      '. The ',
      ui('About'),
      ' menu also offers to ',
      ui('Check for updates'),
      '. Nothing else leaves your machine: the check only asks GitHub whether a new version exists.',
    ),
  ],
};
