import { ErrorHandler, Injectable, inject } from '@angular/core';
import { IpcError } from '@core/ipc/ipc.error';
import { ErrorNotifier, errorDetail } from './error-notifier.service';

/** The last net: on a desktop app an uncaught exception has to reach the screen. */
@Injectable()
export class AppErrorHandler implements ErrorHandler {
  private readonly notifier = inject(ErrorNotifier);

  handleError(error: unknown): void {
    console.error(error);

    if (error instanceof IpcError) {
      this.notifier.notify({
        ref: { key: 'errors.ipcFailed', params: { command: error.command } },
        detail: error.message,
      });
      return;
    }

    this.notifier.notify({ ref: { key: 'errors.unexpected' }, detail: errorDetail(error) });
  }
}
