import { docPath } from '../../doc-index';
import { DocPage, b, field, h2, keys, link, ol, p, shortcuts, table, tip, ui, ul } from '../../doc.model';

export const QUICK_PASTE: DocPage = {
  id: 'paste',
  title: 'Quick paste and shortcuts',
  summary:
    'Finding a snippet and pasting it without leaving what you are doing, capturing the clipboard into a note, and setting the shortcuts.',
  body: [
    h2('palette', 'The quick paste palette'),
    p(
      keys('Ctrl Alt P'),
      ' opens the palette over any application, even when the DevNotes window is closed. It searches all your spaces at once, ignoring the canvas filters: when you recall a snippet, you do not always remember where you filed it.',
    ),
    ol(
      ['Type a few letters of the title, the code or a tag.'],
      ['Choose with the arrows.'],
      [
        keys('Ctrl C'),
        ' copies the note and closes the palette: you are back in the application you came from, ready to paste.',
      ],
    ),
    p(
      'A snippet with fields, like ',
      field('host'),
      ', first asks for their values: copying a command with holes as it is would give a command you cannot use. ',
      keys('Enter'),
      ' or a click opens the note instead of copying it, which also makes the palette a “find this note”.',
    ),
    shortcuts(
      'In the palette',
      ['↑ ↓', 'Go through the results (Tab too)'],
      ['Enter', 'Open the note'],
      ['Ctrl C', 'Copy the note and close'],
      ['Esc', 'Close the palette'],
    ),
    tip('Nothing matches? The last line offers ', ui('+ Create a note “…”'), ' with what you typed.'),
    h2('capture', 'Capturing the clipboard'),
    p(
      keys('Ctrl Alt V'),
      ' does the opposite of the palette: what you have just copied, wherever it was, becomes a note. ',
      keys('Ctrl Alt N'),
      ' opens an empty note. Both work with the window closed.',
    ),
    h2('tray', 'From the system tray'),
    p(
      'The DevNotes icon in the system tray offers ',
      ui('Quick paste'),
      ', ',
      ui('New note'),
      ' and ',
      ui('Paste from clipboard'),
      '. By default, closing the window puts it in the system tray, where DevNotes keeps listening for its shortcuts (',
      ui('Close to tray'),
      ', in the preferences); ',
      ui('Start with the system'),
      ' launches it when you log in.',
    ),
    h2('global', 'Global shortcuts'),
    table(
      'Active everywhere, even with the window closed',
      ['Default', 'Action'],
      [keys('Ctrl Alt P'), 'Open the quick paste palette'],
      [keys('Ctrl Alt V'), 'Capture the clipboard into a note'],
      [keys('Ctrl Alt N'), 'Create a note'],
    ),
    p(
      'These three act in all your applications: they take at least one modifier, or the key would be swallowed everywhere. First come, first served: if another application already holds a combination, DevNotes says so at startup, and you choose another.',
    ),
    h2('areas', 'Switching areas'),
    shortcuts(
      'In all three areas',
      ['Ctrl 1', 'Go to the notes'],
      ['Ctrl 2', 'Go to the tools'],
      ['Ctrl 3', 'Go to the HTTP client'],
      ['Ctrl Shift T', 'Search for a tool'],
    ),
    h2('set', 'Setting your shortcuts'),
    p(
      'In ',
      ui('File → Preferences… → Shortcuts'),
      ', click an action’s field, then press the combination. DevNotes warns you if another action already has it. ',
      ui('Reset'),
      ' goes back to the original.',
    ),
    ul(
      [b('Global: '), 'at least one modifier (Ctrl, Alt, Shift or Super).'],
      [b('Canvas: '), 'a bare letter is enough, since they never act while typing.'],
      [
        b('Fixed: '),
        'the arrows, Esc, Tab and clicks with Ctrl or Shift do not move. They are navigation, leaving and selection.',
      ],
    ),
    p(
      'The full list of shortcuts, wherever you are, is in ',
      ui('About → Keyboard shortcuts'),
      '. The canvas ones are detailed in ',
      link('Organising and finding', docPath('organising', 'en')),
      '.',
    ),
  ],
};
