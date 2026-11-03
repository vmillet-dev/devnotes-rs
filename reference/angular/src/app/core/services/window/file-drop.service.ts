import { InjectionToken, Injectable, inject } from '@angular/core';
import { getCurrentWebview } from '@tauri-apps/api/webview';
import { Unlisten, subscribeCancellable } from '@core/utils/subscription.util';

export type FileDropSubscriber = (handler: (paths: readonly string[]) => void) => Promise<Unlisten>;

/**
 * ⚠️ A drop is a window event, not a DOM one: the WebView never sees the files, only
 * Rust does. An HTML `drop` handler would receive nothing.
 */
export const FILE_DROP_SUBSCRIBER = new InjectionToken<FileDropSubscriber>('FILE_DROP_SUBSCRIBER', {
  providedIn: 'root',
  factory: () => async (handler) =>
    getCurrentWebview().onDragDropEvent((event) => {
      if (event.payload.type === 'drop') {
        handler(event.payload.paths);
      }
    }),
});

@Injectable({ providedIn: 'root' })
export class FileDropService {
  private readonly subscribe = inject(FILE_DROP_SUBSCRIBER);

  on(handler: (paths: readonly string[]) => void): Unlisten {
    return subscribeCancellable(this.subscribe, handler);
  }
}
