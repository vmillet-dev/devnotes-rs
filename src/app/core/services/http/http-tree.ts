import {
  HttpItem,
  HttpItemKind,
  HttpNode,
  HttpPlace,
  HttpRequestSummary,
  HttpTree,
  RequestKind,
  requestBadge,
} from '@core/model/http.model';

/** One line of the rail: the tree laid flat, its level kept as a depth. */
export interface RailRow {
  readonly kind: HttpItemKind;
  readonly id: string;
  readonly name: string;
  readonly depth: number;
  readonly collectionId: string;
  /** The folder it sits in; `null` at a collection's root, and for a collection. */
  readonly folderId: string | null;
  readonly expanded: boolean;
  /** A request's method, or `GQL` and `WS`. */
  readonly badge: string | null;
  readonly requestKind: RequestKind | null;
}

/** Where a gesture sends an item: under a parent at a rank, or a collection among the others. */
export type RailMove =
  | { readonly kind: 'move'; readonly item: HttpItem; readonly place: HttpPlace }
  | { readonly kind: 'reorder'; readonly id: string; readonly index: number };

export type KeyMove = 'up' | 'down' | 'in' | 'out';
export type DropZone = 'before' | 'after' | 'inside';

/** Where a keyboard move left a row, in the terms a reader follows it by. */
export interface Landing {
  /** The folder or collection that now holds it; `null` for a collection, held by nothing. */
  readonly parentId: string | null;
  readonly where: 'into' | 'root' | 'before' | 'after';
  /** The name of what holds it for `into` and `root`, of the row it passed otherwise. */
  readonly other: string;
}

/** Why a keyboard move was not made: said, since a key that does nothing looks unheard. */
export type Refusal = 'first' | 'last' | 'noFolderAbove' | 'atRoot' | 'collection';

interface Located {
  readonly item: HttpItem;
  readonly collectionId: string;
  readonly folderId: string | null;
}

/** The rows a reader sees: what a collapsed collection or folder holds is left out. */
export function railRows(tree: HttpTree, collapsed: ReadonlySet<string>): RailRow[] {
  const rows: RailRow[] = [];
  const walk = (
    children: readonly HttpNode[],
    collectionId: string,
    folderId: string | null,
    depth: number,
  ) => {
    for (const node of children) {
      if (node.kind === 'folder') {
        const expanded = !collapsed.has(node.folder.id);
        rows.push({
          kind: 'folder',
          id: node.folder.id,
          name: node.folder.name,
          depth,
          collectionId,
          folderId,
          expanded,
          badge: null,
          requestKind: null,
        });
        if (expanded) walk(node.children, collectionId, node.folder.id, depth + 1);
      } else {
        rows.push({
          kind: 'request',
          id: node.request.id,
          name: node.request.name,
          depth,
          collectionId,
          folderId,
          expanded: false,
          badge: requestBadge(node.request),
          requestKind: node.request.kind,
        });
      }
    }
  };
  for (const { collection, children } of tree.collections) {
    const expanded = !collapsed.has(collection.id);
    rows.push({
      kind: 'collection',
      id: collection.id,
      name: collection.name,
      depth: 0,
      collectionId: collection.id,
      folderId: null,
      expanded,
      badge: null,
      requestKind: null,
    });
    if (expanded) walk(children, collection.id, null, 1);
  }
  return rows;
}

interface Index {
  readonly located: Map<string, Located>;
  readonly children: Map<string, HttpItem[]>;
  readonly names: Map<string, string>;
}

/** Every item with its parent and its name, and every parent with what it holds, in order. */
function index(tree: HttpTree): Index {
  const located = new Map<string, Located>();
  const children = new Map<string, HttpItem[]>();
  const names = new Map<string, string>();
  const walk = (nodes: readonly HttpNode[], collectionId: string, folderId: string | null) => {
    const items: HttpItem[] = [];
    for (const node of nodes) {
      const item: HttpItem =
        node.kind === 'folder'
          ? { kind: 'folder', id: node.folder.id }
          : { kind: 'request', id: node.request.id };
      items.push(item);
      located.set(item.id, { item, collectionId, folderId });
      names.set(item.id, node.kind === 'folder' ? node.folder.name : node.request.name);
      if (node.kind === 'folder') walk(node.children, collectionId, node.folder.id);
    }
    children.set(folderId ?? collectionId, items);
  };
  for (const { collection, children: nodes } of tree.collections) {
    located.set(collection.id, {
      item: { kind: 'collection', id: collection.id },
      collectionId: collection.id,
      folderId: null,
    });
    names.set(collection.id, collection.name);
    walk(nodes, collection.id, null);
  }
  return { located, children, names };
}

const same = (a: HttpItem, b: HttpItem) => a.kind === b.kind && a.id === b.id;

/** A folder and every folder below it. */
function subtree(children: Map<string, HttpItem[]>, folderId: string): Set<string> {
  const found = new Set([folderId]);
  const visit = (id: string) => {
    for (const child of children.get(id) ?? []) {
      if (child.kind === 'folder') {
        found.add(child.id);
        visit(child.id);
      }
    }
  };
  visit(folderId);
  return found;
}

function collectionOrder(tree: HttpTree): string[] {
  return tree.collections.map((node) => node.collection.id);
}

/** `Alt+↑/↓` within its parent, `Alt+→` into the folder above it, `Alt+←` out of its folder. */
export function keyMove(tree: HttpTree, item: HttpItem, move: KeyMove): RailMove | null {
  if (item.kind === 'collection') {
    const order = collectionOrder(tree);
    const at = order.indexOf(item.id);
    const to = move === 'up' ? at - 1 : move === 'down' ? at + 1 : -1;
    return at < 0 || to < 0 || to >= order.length ? null : { kind: 'reorder', id: item.id, index: to };
  }

  const { located, children } = index(tree);
  const here = located.get(item.id);
  if (!here) return null;
  const siblings = children.get(here.folderId ?? here.collectionId) ?? [];
  const at = siblings.findIndex((sibling) => same(sibling, item));
  const placed = (folderId: string | null, at: number): RailMove => ({
    kind: 'move',
    item,
    place: { collectionId: here.collectionId, folderId, index: at },
  });

  switch (move) {
    case 'up':
      return at > 0 ? placed(here.folderId, at - 1) : null;
    case 'down':
      return at < siblings.length - 1 ? placed(here.folderId, at + 1) : null;
    case 'in': {
      const above = siblings[at - 1];
      return above?.kind === 'folder' ? placed(above.id, (children.get(above.id) ?? []).length) : null;
    }
    case 'out': {
      if (here.folderId === null) return null;
      const parent = located.get(here.folderId);
      if (!parent) return null;
      const around = children.get(parent.folderId ?? parent.collectionId) ?? [];
      return placed(parent.folderId, around.findIndex((sibling) => sibling.id === here.folderId) + 1);
    }
  }
}

/**
 * Read from the tree as it is **after** the move: where `item` now sits, and beside or inside
 * what. `null` when it is no longer there.
 */
export function landing(tree: HttpTree, item: HttpItem, move: KeyMove): Landing | null {
  const { located, children, names } = index(tree);
  const here = located.get(item.id);
  if (!here) return null;

  const collection = item.kind === 'collection';
  const parentId = collection ? null : (here.folderId ?? here.collectionId);
  const around = collection
    ? collectionOrder(tree)
    : (children.get(parentId ?? '') ?? []).map(({ id }) => id);
  const at = around.indexOf(item.id);
  const name = (id: string | undefined) => (id === undefined ? undefined : names.get(id));

  // Up and down are read against the row that was passed: it is the one the eye was on.
  const passed = move === 'up' ? name(around[at + 1]) : move === 'down' ? name(around[at - 1]) : undefined;
  if (passed !== undefined) return { parentId, where: move === 'up' ? 'before' : 'after', other: passed };
  if (parentId === null) return null;
  return {
    parentId,
    where: here.folderId === null ? 'root' : 'into',
    other: names.get(parentId) ?? '',
  };
}

/** Why `keyMove` answered `null`. */
export function refusal(item: HttpItem, move: KeyMove): Refusal {
  if (move === 'up') return 'first';
  if (move === 'down') return 'last';
  if (item.kind === 'collection') return 'collection';
  return move === 'in' ? 'noFolderAbove' : 'atRoot';
}

/**
 * Where a drop on `target` sends `dragged`, or `null` when it would change nothing or cannot
 * be: a folder into itself, a collection among anything but collections.
 */
export function dropMove(
  tree: HttpTree,
  dragged: HttpItem,
  target: HttpItem,
  zone: DropZone,
): RailMove | null {
  if (same(dragged, target)) return null;

  if (dragged.kind === 'collection') {
    if (target.kind !== 'collection') return null;
    const order = collectionOrder(tree).filter((id) => id !== dragged.id);
    const at = order.indexOf(target.id) + (zone === 'before' ? 0 : 1);
    return collectionOrder(tree).indexOf(dragged.id) === at
      ? null
      : { kind: 'reorder', id: dragged.id, index: at };
  }

  const { located, children } = index(tree);
  const from = located.get(dragged.id);
  const onto = located.get(target.id);
  if (!from || !onto) return null;

  const into = target.kind === 'collection' || (target.kind === 'folder' && zone === 'inside');
  const folderId = into ? (target.kind === 'folder' ? target.id : null) : onto.folderId;
  const collectionId = onto.collectionId;
  if (dragged.kind === 'folder' && folderId !== null && subtree(children, dragged.id).has(folderId))
    return null;

  const others = (children.get(folderId ?? collectionId) ?? []).filter((item) => !same(item, dragged));
  const at = into
    ? others.length
    : others.findIndex((item) => same(item, target)) + (zone === 'before' ? 0 : 1);

  const unchanged =
    from.collectionId === collectionId &&
    from.folderId === folderId &&
    (children.get(folderId ?? collectionId) ?? []).findIndex((item) => same(item, dragged)) === at;
  return unchanged ? null : { kind: 'move', item: dragged, place: { collectionId, folderId, index: at } };
}

/** Every request a deletion takes: the request itself, or all a folder or a collection holds. */
export function requestsUnder(tree: HttpTree, item: HttpItem): ReadonlySet<string> {
  const found = new Set<string>();
  const collect = (nodes: readonly HttpNode[]) => {
    for (const node of nodes) {
      if (node.kind === 'request') found.add(node.request.id);
      else collect(node.children);
    }
  };
  const within = (nodes: readonly HttpNode[]): boolean =>
    nodes.some((node) => {
      if (node.kind === 'request') {
        if (item.kind !== 'request' || node.request.id !== item.id) return false;
        found.add(node.request.id);
        return true;
      }
      if (item.kind === 'folder' && node.folder.id === item.id) {
        collect(node.children);
        return true;
      }
      return within(node.children);
    });
  for (const { collection, children } of tree.collections) {
    if (item.kind === 'collection') {
      if (collection.id === item.id) collect(children);
    } else if (within(children)) {
      break;
    }
  }
  return found;
}

/** A request as the tree holds it — its name and where it sits — or `null` once it is gone. */
export function findRequest(tree: HttpTree, requestId: string): HttpRequestSummary | null {
  const walk = (nodes: readonly HttpNode[]): HttpRequestSummary | null => {
    for (const node of nodes) {
      if (node.kind === 'request') {
        if (node.request.id === requestId) return node.request;
      } else {
        const found = walk(node.children);
        if (found) return found;
      }
    }
    return null;
  };
  for (const { children } of tree.collections) {
    const found = walk(children);
    if (found) return found;
  }
  return null;
}

/** Where a request may be saved: each collection, then each of its folders, named by its path. */
export interface SavePlace {
  readonly id: string;
  readonly collectionId: string;
  readonly folderId: string | null;
  readonly path: readonly string[];
}

export function savePlaces(tree: HttpTree): SavePlace[] {
  const places: SavePlace[] = [];
  const walk = (nodes: readonly HttpNode[], collectionId: string, path: readonly string[]) => {
    for (const node of nodes) {
      if (node.kind !== 'folder') continue;
      const here = [...path, node.folder.name];
      places.push({ id: node.folder.id, collectionId, folderId: node.folder.id, path: here });
      walk(node.children, collectionId, here);
    }
  };
  for (const { collection, children } of tree.collections) {
    places.push({ id: collection.id, collectionId: collection.id, folderId: null, path: [collection.name] });
    walk(children, collection.id, [collection.name]);
  }
  return places;
}
