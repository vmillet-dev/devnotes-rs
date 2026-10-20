import { smallMode } from './small-mode';

/** `''…''` is Nix's string over several lines: its quote opens and closes it like `"`. */
export const nix = smallMode({
  name: 'nix',
  keywords: ['let', 'in', 'with', 'rec', 'inherit', 'if', 'then', 'else', 'assert', 'or'],
  types: ['import', 'builtins', 'derivation'],
  atoms: ['true', 'false', 'null'],
  lineComments: ['#'],
  blockComment: ['/*', '*/'],
  quotes: ["''", '"'],
});
