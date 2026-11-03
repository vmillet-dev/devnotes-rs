import { Injectable, inject } from '@angular/core';
import { TranslocoService } from '@jsverse/transloco';
import { commands, type TrayLabels as WireTrayLabels } from '@core/ipc/bindings';

export type TrayLabels = WireTrayLabels;

/** `satisfies` makes a label added to `TrayLabels` in Rust a compile error here. */
const LABEL_KEYS = {
  open: 'tray.open',
  newNote: 'tray.newNote',
  capture: 'tray.capture',
  palette: 'tray.palette',
  quit: 'tray.quit',
} as const satisfies Record<keyof TrayLabels, string>;

/**
 * The front creates the tray rather than the native startup: Rust writes no user-facing
 * text, so without translated labels there would be nothing to show. The subscription
 * re-emits on every language change, which re-translates the menu.
 */
@Injectable({ providedIn: 'root' })
export class TrayService {
  private readonly transloco = inject(TranslocoService);

  start(): void {
    this.transloco
      .selectTranslate<string[]>(Object.values(LABEL_KEYS))
      .subscribe(([open = '', newNote = '', capture = '', palette = '', quit = '']) => {
        void this.push({ open, newNote, capture, palette, quit });
      });
  }

  /** A failure does not surface: on a desktop without a tray, Rust declines silently. */
  private async push(labels: TrayLabels): Promise<void> {
    try {
      // The one command with no `Result` on the Rust side, hence no `unwrap`.
      await commands.syncTray(labels);
    } catch {
      // Without a tray, the application lives in its window.
    }
  }
}
