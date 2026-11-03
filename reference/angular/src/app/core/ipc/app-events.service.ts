import { InjectionToken, Injectable, inject } from '@angular/core';
import { listen } from '@tauri-apps/api/event';
import { Unlisten, subscribeCancellable } from '@core/utils/subscription.util';
import { GLOBAL_ACTION_EVENT, GlobalAction } from './bindings';

export type { GlobalAction };

export type EventSubscriber = (handler: (action: GlobalAction) => void) => Promise<Unlisten>;

export const EVENT_SUBSCRIBER = new InjectionToken<EventSubscriber>('EVENT_SUBSCRIBER', {
  providedIn: 'root',
  factory: () => (handler) => listen<GlobalAction>(GLOBAL_ACTION_EVENT, (event) => handler(event.payload)),
});

/**
 * The bridge's downward direction. One topic carrying a closed, generated action rather
 * than one topic per action: a topic string spelled on both sides makes a typo into a
 * subscription that is silently inert. Outside Tauri `listen` fails, inertly.
 */
@Injectable({ providedIn: 'root' })
export class AppEventsService {
  private readonly subscribe = inject(EVENT_SUBSCRIBER);

  on(handler: (action: GlobalAction) => void): Unlisten {
    return subscribeCancellable(this.subscribe, handler);
  }
}
