import { AvailableUpdate, DownloadProgress } from '@core/services/updates/updater.service';

/** The real one reaches for the Tauri bridge, absent under jsdom. */
export class FakeUpdater {
  /** What the next `check()` resolves to. */
  available: AvailableUpdate | null = null;
  checkError: Error | null = null;
  installError: Error | null = null;
  /** Progress values replayed to the callback on `install()`. */
  progressSteps: DownloadProgress[] = [null, 1];

  checkCalls = 0;
  installCalls = 0;
  relaunched = false;
  discarded = false;

  /** Hold `install()` / `check()` open so a spec can observe the in-flight state. */
  deferInstall = false;
  deferCheck = false;
  private release: (() => void) | null = null;
  private releaseCheck: (() => void) | null = null;

  async check(): Promise<AvailableUpdate | null> {
    this.checkCalls += 1;

    if (this.deferCheck) {
      await new Promise<void>((resolve) => {
        this.releaseCheck = resolve;
      });
    }
    if (this.checkError) throw this.checkError;
    return this.available;
  }

  finishCheck(): void {
    this.releaseCheck?.();
    this.releaseCheck = null;
  }

  async install(onProgress: (progress: DownloadProgress) => void): Promise<void> {
    this.installCalls += 1;
    for (const step of this.progressSteps) onProgress(step);

    if (this.deferInstall) {
      await new Promise<void>((resolve) => {
        this.release = resolve;
      });
    }
    if (this.installError) throw this.installError;
  }

  finishInstall(): void {
    this.release?.();
    this.release = null;
  }

  async relaunch(): Promise<void> {
    this.relaunched = true;
  }

  async discard(): Promise<void> {
    this.discarded = true;
  }
}
