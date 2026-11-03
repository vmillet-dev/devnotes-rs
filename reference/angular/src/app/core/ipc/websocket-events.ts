import { InjectionToken } from '@angular/core';
import { listen } from '@tauri-apps/api/event';
import { Unlisten } from '@core/utils/subscription.util';
import { WEBSOCKET_EVENT, WebsocketEvent } from './bindings';

export type WebsocketSubscriber = (handler: (event: WebsocketEvent) => void) => Promise<Unlisten>;

/** The sockets' one topic, its name and payload generated: a token, so a spec can play events. */
export const WEBSOCKET_SUBSCRIBER = new InjectionToken<WebsocketSubscriber>('WEBSOCKET_SUBSCRIBER', {
  providedIn: 'root',
  factory: () => (handler) => listen<WebsocketEvent>(WEBSOCKET_EVENT, (event) => handler(event.payload)),
});
