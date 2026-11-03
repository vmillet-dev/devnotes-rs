import { DesktopNotificationAdapter } from '@core/services/notifications/desktop-notifier.service';
import { FailsNext, guard } from './fail-next';

/** Under jsdom the real plugin rejects, and `DesktopNotifier` answers `false` in silence. */
export class FakeDesktopNotifications implements DesktopNotificationAdapter, FailsNext {
  /** Every toast it was handed, in order, so a spec can assert what was said. */
  readonly sent: { title: string; body: string }[] = [];

  /** What the desktop answers when asked. `'denied'` is a state, not a failure. */
  permission: NotificationPermission;

  /** How many times permission was asked for: once is the rule, not once per copy. */
  asked = 0;

  /** When set, the next call rejects like the plugin does outside Tauri. */
  failNext: Error | null = null;

  constructor(permission: NotificationPermission = 'granted') {
    this.permission = permission;
  }

  isPermissionGranted(): Promise<boolean> {
    return guard(this, () => this.permission === 'granted');
  }

  requestPermission(): Promise<NotificationPermission> {
    return guard(this, () => {
      this.asked += 1;
      return this.permission;
    });
  }

  send(options: { title: string; body: string }): void {
    this.sent.push(options);
  }
}
