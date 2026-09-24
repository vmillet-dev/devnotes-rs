import { Injectable, inject } from '@angular/core';
import { ClipboardService } from '@core/services/clipboard/clipboard.service';
import { ErrorNotifier } from '@core/services/errors/error-notifier.service';

@Injectable({ providedIn: 'root' })
export class NoteCopyService {
  private readonly clipboard = inject(ClipboardService);
  private readonly notifier = inject(ErrorNotifier);

  /** Answers what the clipboard actually accepted: an acknowledgement is earned. */
  async copy(content: string): Promise<boolean> {
    if (await this.clipboard.copy(content)) return true;

    this.notifier.notify({ ref: { key: 'errors.copyFailed' } });
    return false;
  }
}
