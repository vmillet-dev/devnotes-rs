import type { CodeTarget } from '@core/ipc/bindings';

export type { CodeNotice, CodeOptions, CodeTarget, GeneratedCode } from '@core/ipc/bindings';

/** The groups the list is cut in, in its order: their names are translated. */
export type CodeGroup = 'shell' | 'javascript' | 'other';

/** A target as the list shows it: its name and its library are proper names, never translated. */
export interface CodeTargetEntry {
  readonly id: CodeTarget;
  readonly name: string;
  readonly library: string | null;
  readonly group: CodeGroup;
}

/**
 * `Record` and not a list alone: a target added in Rust breaks this build until it is named.
 * The key order is the list's.
 */
const ENTRIES: Record<CodeTarget, Omit<CodeTargetEntry, 'id'>> = {
  curl: { name: 'cURL', library: null, group: 'shell' },
  httpie: { name: 'HTTPie', library: null, group: 'shell' },
  wget: { name: 'wget', library: null, group: 'shell' },
  powershell: { name: 'PowerShell', library: 'Invoke-RestMethod', group: 'shell' },
  fetch: { name: 'JavaScript', library: 'fetch', group: 'javascript' },
  axios: { name: 'JavaScript', library: 'axios', group: 'javascript' },
  undici: { name: 'Node.js', library: 'undici', group: 'javascript' },
  angular: { name: 'TypeScript', library: 'Angular HttpClient', group: 'javascript' },
};

export const CODE_TARGETS: readonly CodeTargetEntry[] = (Object.keys(ENTRIES) as CodeTarget[]).map((id) => ({
  id,
  ...ENTRIES[id],
}));

export const CODE_GROUPS: readonly CodeGroup[] = ['shell', 'javascript', 'other'];

/** Its name or its library holds what is typed, case and accents aside. */
export function matchesFilter(entry: CodeTargetEntry, filter: string): boolean {
  const fold = (text: string) =>
    text
      .normalize('NFD')
      .replace(/\p{Diacritic}/gu, '')
      .toLowerCase();
  const wanted = fold(filter.trim());
  return wanted === '' || fold(`${entry.name} ${entry.library ?? ''}`).includes(wanted);
}
