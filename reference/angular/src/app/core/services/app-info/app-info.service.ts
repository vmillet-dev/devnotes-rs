import { Injectable, Signal, computed, resource } from '@angular/core';
import { getTauriVersion, getVersion } from '@tauri-apps/api/app';
import { openUrl } from '@tauri-apps/plugin-opener';
import { APP_METADATA } from '@core/ipc/bindings';

/** Generated from `Cargo.toml` at compile time. */
export const APP_INFO = APP_METADATA;

@Injectable({ providedIn: 'root' })
export class AppInfoService {
  private readonly versionResource = resource({ loader: () => getVersion() });
  private readonly tauriResource = resource({ loader: () => getTauriVersion() });

  /** `null` outside the Tauri runtime: the card then shows a dash rather than lying. */
  readonly version: Signal<string | null> = computed(() =>
    this.versionResource.hasValue() ? this.versionResource.value() : null,
  );

  /** Asked of the running framework rather than baked in: a committed `bindings.ts` can lag. */
  readonly tauriVersion: Signal<string | null> = computed(() =>
    this.tauriResource.hasValue() ? this.tauriResource.value() : null,
  );

  async openRepository(): Promise<void> {
    await openUrl(APP_INFO.repository);
  }
}
