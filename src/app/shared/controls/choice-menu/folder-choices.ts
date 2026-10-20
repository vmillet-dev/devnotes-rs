import { Folder, treeOrder } from '@core/model/folder.model';
import type { ChoiceOption } from './choice-menu.component';

/** One space's folders as a menu offers them: the tree in order, a nested one hinted by its path. */
export function folderChoices(folders: readonly Folder[]): ChoiceOption[] {
  return treeOrder(folders).map(({ folder, path }) => ({
    id: folder.id,
    name: folder.name,
    colour: folder.colour,
    ...(path.length > 1 && { hint: path.map((each) => each.name).join(' › ') }),
  }));
}
