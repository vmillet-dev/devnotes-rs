import { Injectable } from '@angular/core';
import { commands } from '@core/ipc/bindings';
import { unwrap } from '@core/ipc/ipc.error';
import {
  EnvironmentPatch,
  EnvironmentSets,
  HttpEnvironment,
  VariableFault,
  VariableRow,
  toEnvironment,
  toVariableRow,
} from '@core/model/http-environment.model';

/** The globals and the environments a request resolves its `{{variables}}` from: the library's. */
@Injectable({ providedIn: 'root' })
export class HttpEnvironmentsRepository {
  async list(): Promise<EnvironmentSets> {
    const sets = unwrap('http_environments', await commands.httpEnvironments());
    return {
      globals: sets.globals.map(toVariableRow),
      environments: sets.environments.map(toEnvironment),
    };
  }

  async create(name: string): Promise<HttpEnvironment> {
    return toEnvironment(unwrap('create_http_environment', await commands.createHttpEnvironment(name)));
  }

  async save(id: string, patch: EnvironmentPatch): Promise<HttpEnvironment> {
    return toEnvironment(unwrap('save_http_environment', await commands.saveHttpEnvironment(id, patch)));
  }

  async saveGlobals(variables: readonly VariableRow[]): Promise<VariableRow[]> {
    return unwrap('save_http_globals', await commands.saveHttpGlobals([...variables])).map(toVariableRow);
  }

  /** `name` is the front's: « (copie) » is a word. */
  async duplicate(id: string, name: string): Promise<HttpEnvironment> {
    return toEnvironment(
      unwrap('duplicate_http_environment', await commands.duplicateHttpEnvironment(id, name)),
    );
  }

  async delete(id: string): Promise<void> {
    unwrap('delete_http_environment', await commands.deleteHttpEnvironment(id));
  }

  /** What stops each variable from being used, by its rank: the rule is Rust's. */
  async check(variables: readonly VariableRow[]): Promise<(VariableFault | null)[]> {
    return unwrap('check_http_variables', await commands.checkHttpVariables([...variables]));
  }
}
