import type {
  Environment as WireEnvironment,
  FilledBy,
  FolderColour,
  HttpMethod,
  RequestKind,
  Variable,
  VariableKind,
} from '@core/ipc/bindings';

export type {
  EnvironmentPatch,
  FilledBy,
  ScopedVariable,
  UsedVariable,
  VariableFault,
  VariableKind,
  VariableScope,
} from '@core/ipc/bindings';

/** A variable, every field present: the wire leaves out a default. */
export interface VariableRow {
  readonly enabled: boolean;
  readonly name: string;
  readonly value: string;
  readonly kind: VariableKind;
  /** The request whose last send wrote it: « remplie par Login ». Absent when none did. */
  readonly filledBy?: FilledBy;
}

export interface HttpEnvironment {
  readonly id: string;
  readonly name: string;
  readonly colour: FolderColour;
  readonly protected: boolean;
  readonly variables: readonly VariableRow[];
}

export interface EnvironmentSets {
  readonly globals: readonly VariableRow[];
  readonly environments: readonly HttpEnvironment[];
}

/** What the page shows on its right: the globals, or one environment. */
export type EnvironmentTarget =
  { readonly kind: 'globals' } | { readonly kind: 'environment'; readonly id: string };

export const VARIABLE_KINDS: readonly VariableKind[] = ['text', 'number', 'secret'];

/** The palette a new environment is given from, in `FolderColour::ALL`'s order. */
export const ENVIRONMENT_COLOURS: readonly FolderColour[] = ['blue', 'amber', 'purple', 'green', 'red'];

export function toVariableRow(variable: Variable): VariableRow {
  return {
    enabled: variable.enabled ?? true,
    name: variable.name ?? '',
    value: variable.value ?? '',
    kind: variable.kind ?? 'text',
    ...(variable.filledBy ? { filledBy: variable.filledBy } : {}),
  };
}

export function toEnvironment(environment: WireEnvironment): HttpEnvironment {
  return {
    id: environment.id,
    name: environment.name,
    colour: environment.colour,
    protected: environment.protected,
    variables: environment.variables.map(toVariableRow),
  };
}

/** What the list says under a name: how many, and how many of them secret. */
export function variableCounts(variables: readonly VariableRow[]): {
  readonly count: number;
  readonly secrets: number;
} {
  const named = variables.filter((variable) => variable.name.trim() !== '');
  return { count: named.length, secrets: named.filter((variable) => variable.kind === 'secret').length };
}

/**
 * A protected environment lets through, unasked, what reads without changing anything; a
 * socket asks before it opens, whatever it then says.
 */
export function asksBeforeSending(kind: RequestKind, method: HttpMethod): boolean {
  return kind === 'websocket' || !['GET', 'HEAD', 'OPTIONS'].includes(method);
}
