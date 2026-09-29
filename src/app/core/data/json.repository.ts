import { Injectable } from '@angular/core';
import { commands } from '@core/ipc/bindings';
import { unwrap } from '@core/ipc/ipc.error';
import { JsonQuery, JsonView } from '@core/model/json.model';

/** A text, never a note id: an HTTP response is explored the same way. */
@Injectable({ providedIn: 'root' })
export class JsonRepository {
  async explore(query: JsonQuery): Promise<JsonView> {
    return unwrap('explore_json', await commands.exploreJson(query));
  }
}
