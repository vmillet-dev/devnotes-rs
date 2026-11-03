import { Signal, signal } from '@angular/core';
import { AppInfoService } from '@core/services/app-info/app-info.service';

/** The real one calls `getVersion()` eagerly, which needs the bridge jsdom lacks. */
export class FakeAppInfo implements Pick<AppInfoService, 'version' | 'tauriVersion' | 'openRepository'> {
  /** Writable here so a spec can flip it to `null` and assert the fallback. */
  readonly versionSignal = signal<string | null>('0.1.0');
  readonly version: Signal<string | null> = this.versionSignal.asReadonly();

  readonly tauriVersionSignal = signal<string | null>('2.0.0');
  readonly tauriVersion: Signal<string | null> = this.tauriVersionSignal.asReadonly();

  openedRepository = 0;
  openError: Error | null = null;

  async openRepository(): Promise<void> {
    this.openedRepository += 1;
    if (this.openError) throw this.openError;
  }
}
