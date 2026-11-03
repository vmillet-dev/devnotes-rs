import { FIELD_NAME_PATTERN } from '@core/ipc/bindings';

/** A pair and not a `Record` entry: a half-filled row has to exist long enough to be corrected. */
export interface Variable {
  readonly name: string;
  readonly value: string;
}

/**
 * The rule lives in `notes::placeholder`, and travels as `FIELD_NAME_PATTERN`: this
 * refuses a name in the same terms rather than keeping a copy of them. It exists only to
 * say so before a badly named row vanishes on save without a word — the back end refuses
 * it either way.
 */
const FIELD_NAME = new RegExp(FIELD_NAME_PATTERN);

export function isVariableName(name: string): boolean {
  return FIELD_NAME.test(name);
}

/** Last one wins on a duplicate, as the JSON object crossing the bridge would. */
export function toVariableRecord(variables: readonly Variable[]): Record<string, string> {
  const record: Record<string, string> = {};

  for (const { name, value } of variables) {
    if (isVariableName(name) && value !== '') {
      record[name] = value;
    }
  }

  return record;
}

/** Names carried by more than one row: the panel flags them. */
export function duplicateNames(variables: readonly Variable[]): ReadonlySet<string> {
  const seen = new Set<string>();
  const duplicates = new Set<string>();

  for (const { name } of variables) {
    if (name === '') continue;

    if (seen.has(name)) {
      duplicates.add(name);
    }
    seen.add(name);
  }

  return duplicates;
}
