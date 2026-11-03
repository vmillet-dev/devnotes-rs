import { InjectionToken } from '@angular/core';
import { listen } from '@tauri-apps/api/event';
import { Unlisten } from '@core/utils/subscription.util';
import { ConsoleEvent, HTTP_CONSOLE_EVENT } from './bindings';

export type HttpConsoleSubscriber = (handler: (event: ConsoleEvent) => void) => Promise<Unlisten>;

/** The console's one topic, its name and payload generated: a token, so a spec can play lines. */
export const HTTP_CONSOLE_SUBSCRIBER = new InjectionToken<HttpConsoleSubscriber>('HTTP_CONSOLE_SUBSCRIBER', {
  providedIn: 'root',
  factory: () => (handler) => listen<ConsoleEvent>(HTTP_CONSOLE_EVENT, (event) => handler(event.payload)),
});
