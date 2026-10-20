import { InjectionToken } from '@angular/core';
import { listen } from '@tauri-apps/api/event';
import { Unlisten } from '@core/utils/subscription.util';
import { HTTP_RUN_EVENT, RunEvent } from './bindings';

export type HttpRunSubscriber = (handler: (event: RunEvent) => void) => Promise<Unlisten>;

/** A run's one topic, its name and payload generated: a token, so a spec can play its steps. */
export const HTTP_RUN_SUBSCRIBER = new InjectionToken<HttpRunSubscriber>('HTTP_RUN_SUBSCRIBER', {
  providedIn: 'root',
  factory: () => (handler) => listen<RunEvent>(HTTP_RUN_EVENT, (event) => handler(event.payload)),
});
