import type { Folder as WireFolder } from '@core/ipc/bindings';

/** Generated, so a colour added in Rust is a compile error here rather than a blank swatch. */
export type { FolderColour, FolderMove, NoteFolder, NoteFiling } from '@core/ipc/bindings';

/**
 * `colour` carries `#[serde(default)]` on the Rust side so an export file written
 * before folders stays readable, and specta turns that into an optional key — which the
 * model refuses to be. Same seam as `Space.pinned`, and `parentId` the same again.
 */
export interface Folder {
  readonly id: string;
  readonly spaceId: string;
  readonly name: string;
  readonly colour: NonNullable<WireFolder['colour']>;
  readonly createdAt: Date;
  /** `null` at the top of its space. */
  readonly parentId: string | null;
}

export interface FolderDraft {
  readonly spaceId: string;
  readonly name: string;
  readonly parentId?: string | null;
}

/** By name, as the rail and « Ranger » list them: siblings have no order of their own. */
export function childrenOf(folders: readonly Folder[], parentId: string | null): Folder[] {
  return folders
    .filter((folder) => folder.parentId === parentId)
    .sort((a, b) => a.name.localeCompare(b.name, undefined, { sensitivity: 'base' }));
}

/** From the top of its space down to the folder itself; empty for a folder not in the list. */
export function pathOf(folders: readonly Folder[], id: string): Folder[] {
  const byId = new Map(folders.map((folder) => [folder.id, folder]));
  const path: Folder[] = [];
  for (let at = byId.get(id); at && !path.includes(at); at = byId.get(at.parentId ?? '')) {
    path.unshift(at);
  }
  return path;
}

/** The folder and every folder under it. */
export function subtreeOf(folders: readonly Folder[], id: string): ReadonlySet<string> {
  const found = new Set([id]);
  for (let grew = true; grew;) {
    grew = false;
    for (const folder of folders) {
      if (folder.parentId !== null && found.has(folder.parentId) && !found.has(folder.id)) {
        found.add(folder.id);
        grew = true;
      }
    }
  }
  return found;
}

/**
 * Whether a drop may put `id` under `parentId`: in its own space, never into itself or what it
 * holds, and somewhere else than where it is. Rust checks the same, and the depth besides.
 */
export function canMoveFolder(folders: readonly Folder[], id: string, parentId: string | null): boolean {
  const folder = folders.find((candidate) => candidate.id === id);
  if (!folder || folder.parentId === parentId) return false;
  if (parentId === null) return true;

  const parent = folders.find((candidate) => candidate.id === parentId);
  return parent !== undefined && parent.spaceId === folder.spaceId && !subtreeOf(folders, id).has(parentId);
}

/** The whole tree depth first, siblings by name, each folder with the path down to it. */
export function treeOrder(
  folders: readonly Folder[],
): { readonly folder: Folder; readonly path: Folder[] }[] {
  const ordered: { folder: Folder; path: Folder[] }[] = [];
  const visit = (parentId: string | null, above: Folder[]) => {
    for (const folder of childrenOf(folders, parentId)) {
      if (above.includes(folder)) continue;
      const path = [...above, folder];
      ordered.push({ folder, path });
      visit(folder.id, path);
    }
  };
  visit(null, []);
  return ordered;
}
