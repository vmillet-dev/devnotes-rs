import { Injectable } from '@angular/core';
import { openUrl } from '@tauri-apps/plugin-opener';
import { commands } from '@core/ipc/bindings';
import type { ChangelogRelease, ChangelogSection, ChangelogSpan } from '@core/ipc/bindings';
import { APP_INFO } from './app-info.service';

export type { ChangelogRelease, ChangelogSection, ChangelogSpan };

/**
 * ⚠️ Must stay covered by the scope declared for `opener:allow-open-url` in
 * `src-tauri/capabilities/default.json`, or opening is refused at runtime.
 */
export const RELEASES_URL = `${APP_INFO.repository}/releases`;

@Injectable({ providedIn: 'root' })
export class ChangelogService {
  /** Newest release first. Rejects when there is no bridge, so the dialog can say so. */
  async load(): Promise<readonly ChangelogRelease[]> {
    // No `Result` on the Rust side: reading an embedded string cannot fail.
    return commands.appChangelog();
  }

  async openReleases(): Promise<void> {
    await openUrl(RELEASES_URL);
  }
}
