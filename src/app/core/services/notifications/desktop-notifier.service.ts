import { InjectionToken, Injectable, inject } from '@angular/core';
import { isPermissionGranted, requestPermission, sendNotification } from '@tauri-apps/plugin-notification';
import { TranslocoService } from '@jsverse/transloco';
import { TranslationRef } from '../i18n/translation-ref.model';

/** A token rather than a direct call, for the same reason as `CLIPBOARD_ADAPTER`. */
export interface DesktopNotificationAdapter {
  isPermissionGranted(): Promise<boolean>;
  requestPermission(): Promise<NotificationPermission>;
  send(options: { title: string; body: string }): void;
}

export const DESKTOP_NOTIFICATION_ADAPTER = new InjectionToken<DesktopNotificationAdapter>(
  'DESKTOP_NOTIFICATION_ADAPTER',
  {
    providedIn: 'root',
    factory: () => ({ isPermissionGranted, requestPermission, send: sendNotification }),
  },
);

/**
 * A toast on the desktop, for the one path whose acknowledgement cannot be drawn in the
 * window: the palette copies and then hides it.
 *
 * ⚠️ Never load-bearing, and never in the way. Whatever it is asked to say has already
 * happened, so a refused permission, a plugin that is not there and a desktop that drops
 * the toast all come to the same thing — `false`, and nothing thrown. `StatusNotifier` is
 * still the answer everywhere the window stays.
 *
 * ⚠️ It translates, where the rest of the application hands a `TranslationRef` to the
 * `transloco` pipe. There is no template on the other side of this one: the string leaves
 * for the operating system already formed.
 */
@Injectable({ providedIn: 'root' })
export class DesktopNotifier {
  private readonly adapter = inject(DESKTOP_NOTIFICATION_ADAPTER);
  private readonly transloco = inject(TranslocoService);

  /** Asked once and remembered: the prompt must not arrive on every copy. */
  private granted: boolean | null = null;

  async notify(title: TranslationRef, body: TranslationRef): Promise<boolean> {
    if (!(await this.allowed())) return false;

    try {
      this.adapter.send({ title: this.say(title), body: this.say(body) });
      return true;
    } catch {
      return false;
    }
  }

  private async allowed(): Promise<boolean> {
    if (this.granted !== null) return this.granted;

    try {
      this.granted = (await this.adapter.isPermissionGranted())
        ? true
        : (await this.adapter.requestPermission()) === 'granted';
    } catch {
      // Outside Tauri the plugin throws rather than answering no.
      this.granted = false;
    }

    return this.granted;
  }

  private say(ref: TranslationRef): string {
    return this.transloco.translate(ref.key, ref.params);
  }
}
