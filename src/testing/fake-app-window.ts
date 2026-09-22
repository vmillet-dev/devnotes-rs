import { AppWindowAdapter } from '@core/services/window/app-window.service';

/** Substituted in every spec: a real `exit()` would take the test runner down. */
export class FakeAppWindow implements AppWindowAdapter {
  hidden = 0;
  exitedWith: number | null = null;
  reloaded = 0;

  throwOnHide: Error | null = null;
  throwOnExit: Error | null = null;

  async hide(): Promise<void> {
    if (this.throwOnHide) throw this.throwOnHide;
    this.hidden += 1;
  }

  async exit(code: number): Promise<void> {
    if (this.throwOnExit) throw this.throwOnExit;
    this.exitedWith = code;
  }

  reload(): void {
    this.reloaded += 1;
  }
}
