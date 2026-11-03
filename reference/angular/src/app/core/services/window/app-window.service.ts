import { InjectionToken, Injectable, inject } from '@angular/core';
import { exit } from '@tauri-apps/plugin-process';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { commands } from '@core/ipc/bindings';

/** A token: a spec really calling `exit()` would take the test runner down. */
export interface AppWindowAdapter {
  hide(): Promise<void>;
  exit(code: number): Promise<void>;
  reload(): void;
  holdQuit(): Promise<void>;
  isFullscreen(): Promise<boolean>;
  setFullscreen(fullscreen: boolean): Promise<void>;
}

export const APP_WINDOW_ADAPTER = new InjectionToken<AppWindowAdapter>('APP_WINDOW_ADAPTER', {
  providedIn: 'root',
  factory: () => ({
    hide: () => getCurrentWindow().hide(),
    exit: (code: number) => exit(code),
    reload: () => window.location.reload(),
    holdQuit: () => commands.holdQuit(),
    isFullscreen: () => getCurrentWindow().isFullscreen(),
    setFullscreen: (fullscreen: boolean) => getCurrentWindow().setFullscreen(fullscreen),
  }),
});

/**
 * The two are distinct: the close button hides (`lib.rs` intercepts `CloseRequested`
 * while there is a tray), and `quit` is the only path that ends the process.
 */
@Injectable({ providedIn: 'root' })
export class AppWindowService {
  private readonly adapter = inject(APP_WINDOW_ADAPTER);

  /** After a copy from the palette: the user goes off to paste elsewhere. */
  async hide(): Promise<void> {
    try {
      await this.adapter.hide();
    } catch {
      // Outside Tauri, or the window is already hidden: nothing to recover.
    }
  }

  /**
   * Rebuilds the front end from nothing, over the same process. What a library switch
   * needs: every store is `providedIn: 'root'` and would otherwise carry the other
   * library's spaces, selection and undo record into this one.
   */
  reload(): void {
    this.adapter.reload();
  }

  /**
   * Answers a quit asked from outside the page — the tray, the window's close button — when
   * there is something to ask first: Rust stops waiting, and shows the window, which may be
   * hidden in the tray where no dialog is seen.
   */
  async holdQuit(): Promise<void> {
    try {
      await this.adapter.holdQuit();
    } catch {
      // Outside Tauri there is no quit to hold.
    }
  }

  /**
   * The screen given to a Note presented. Answers whether the window changed, so the one that
   * asked gives back only what it took: a window already full screen stays so.
   */
  async enterFullscreen(): Promise<boolean> {
    try {
      if (await this.adapter.isFullscreen()) return false;
      await this.adapter.setFullscreen(true);
      return true;
    } catch {
      // Outside Tauri: the presentation fills the window instead.
      return false;
    }
  }

  async leaveFullscreen(): Promise<void> {
    try {
      await this.adapter.setFullscreen(false);
    } catch {
      // The window is closing, or this is not Tauri: nothing is left to give back.
    }
  }

  async quit(): Promise<void> {
    try {
      await this.adapter.exit(0);
    } catch {
      // A failure here leaves nothing inconsistent behind.
    }
  }
}
