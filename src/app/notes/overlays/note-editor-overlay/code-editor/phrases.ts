/**
 * CodeMirror's own words still on screen or spoken — the search bar is ours and has its own — and
 * the key each is translated under.
 */
const PHRASE_KEYS: Readonly<Record<string, string>> = {
  close: 'close',
  'current match': 'currentMatch',
  'on line': 'onLine',
  'replaced $ matches': 'replacedMatches',
  'replaced match on line $': 'replacedOnLine',
  'Go to line': 'goToLine',
  go: 'go',
  'Control character': 'controlCharacter',
  'Selection deleted': 'selectionDeleted',
};

/** From the `editor.code` translations to `EditorState.phrases`; a missing one stays English. */
export function phrasesFrom(translated: Readonly<Record<string, unknown>>): Record<string, string> {
  const phrases: Record<string, string> = {};
  for (const [phrase, key] of Object.entries(PHRASE_KEYS)) {
    const text = translated[key];
    if (typeof text === 'string' && text !== '') phrases[phrase] = text;
  }
  return phrases;
}
