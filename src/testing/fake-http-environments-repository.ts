import { HttpEnvironmentsRepository } from '@core/data/http-environments.repository';
import {
  ENVIRONMENT_COLOURS,
  EnvironmentPatch,
  EnvironmentSets,
  HttpEnvironment,
  VariableFault,
  VariableRow,
} from '@core/model/http-environment.model';
import { guard } from './fail-next';

export interface EnvironmentsCall {
  readonly command: string;
  readonly args: readonly unknown[];
}

export function environment(name: string, overrides: Partial<HttpEnvironment> = {}): HttpEnvironment {
  return {
    id: `env-${name.toLowerCase()}`,
    name,
    colour: 'blue',
    protected: false,
    variables: [],
    ...overrides,
  };
}

export function variable(name: string, value: string, overrides: Partial<VariableRow> = {}): VariableRow {
  return { enabled: true, name, value, kind: 'text', ...overrides };
}

/**
 * Keeps what it is given and answers it back, recording each call. The rules — a name held
 * twice, a fault — are Rust's: a spec sets `failNext` or `faults` to what Rust would answer.
 */
export class FakeHttpEnvironmentsRepository implements Pick<
  HttpEnvironmentsRepository,
  keyof HttpEnvironmentsRepository
> {
  sets: EnvironmentSets = { globals: [], environments: [] };
  faults: (VariableFault | null)[] = [];
  failNext: Error | null = null;
  readonly calls: EnvironmentsCall[] = [];
  private made = 0;

  constructor(sets: Partial<EnvironmentSets> = {}) {
    this.sets = { globals: [], environments: [], ...sets };
  }

  callsTo(command: string): readonly EnvironmentsCall[] {
    return this.calls.filter((call) => call.command === command);
  }

  list(): Promise<EnvironmentSets> {
    return this.record('list', [], () => this.sets);
  }

  create(name: string): Promise<HttpEnvironment> {
    return this.record('create', [name], () => {
      const created: HttpEnvironment = {
        id: `env-new-${++this.made}`,
        name,
        colour: ENVIRONMENT_COLOURS[this.sets.environments.length % ENVIRONMENT_COLOURS.length] ?? 'blue',
        protected: false,
        variables: [],
      };
      this.sets = { ...this.sets, environments: [...this.sets.environments, created] };
      return created;
    });
  }

  save(id: string, patch: EnvironmentPatch): Promise<HttpEnvironment> {
    return this.record('save', [id, patch], () => {
      const held = this.sets.environments.find((environment) => environment.id === id);
      if (held === undefined) throw new Error(`no environment ${id}`);
      const saved: HttpEnvironment = {
        ...held,
        ...(patch.name == null ? {} : { name: patch.name }),
        ...(patch.colour == null ? {} : { colour: patch.colour }),
        ...(patch.protected == null ? {} : { protected: patch.protected }),
        ...(patch.variables == null ? {} : { variables: patch.variables as VariableRow[] }),
      };
      this.sets = {
        ...this.sets,
        environments: this.sets.environments.map((environment) =>
          environment.id === id ? saved : environment,
        ),
      };
      return saved;
    });
  }

  saveGlobals(variables: readonly VariableRow[]): Promise<VariableRow[]> {
    return this.record('saveGlobals', [variables], () => {
      this.sets = { ...this.sets, globals: [...variables] };
      return [...variables];
    });
  }

  duplicate(id: string, name: string): Promise<HttpEnvironment> {
    return this.record('duplicate', [id, name], () => {
      const at = this.sets.environments.findIndex((environment) => environment.id === id);
      const original = this.sets.environments[at];
      if (original === undefined) throw new Error(`no environment ${id}`);
      const copy: HttpEnvironment = { ...original, id: `env-copy-${++this.made}`, name };
      const environments = [...this.sets.environments];
      environments.splice(at + 1, 0, copy);
      this.sets = { ...this.sets, environments };
      return copy;
    });
  }

  delete(id: string): Promise<void> {
    return this.record('delete', [id], () => {
      this.sets = {
        ...this.sets,
        environments: this.sets.environments.filter((environment) => environment.id !== id),
      };
    });
  }

  check(variables: readonly VariableRow[]): Promise<(VariableFault | null)[]> {
    return this.record('check', [variables], () => this.faults);
  }

  private record<T>(command: string, args: readonly unknown[], answer: () => T): Promise<T> {
    this.calls.push({ command, args });
    return guard(this, answer);
  }
}
