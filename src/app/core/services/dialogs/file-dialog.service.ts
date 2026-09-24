import { InjectionToken, Injectable, inject } from '@angular/core';
import { open, save } from '@tauri-apps/plugin-dialog';
import type { OpenDialogOptions, SaveDialogOptions } from '@tauri-apps/plugin-dialog';
import { APP_INFO } from '@core/services/app-info/app-info.service';

/** A token rather than a direct call, for the same reason as `CLIPBOARD_ADAPTER`. */
export interface FileDialogAdapter {
  open(options: OpenDialogOptions): Promise<string | string[] | null>;
  save(options: SaveDialogOptions): Promise<string | null>;
}

export const FILE_DIALOG_ADAPTER = new InjectionToken<FileDialogAdapter>('FILE_DIALOG_ADAPTER', {
  providedIn: 'root',
  factory: () => ({ open, save }),
});

/** `json` stays on the way in: an export written before the archive existed is still
 * importable, and the picker has to let the user reach it. */
const OPEN_FILTER = { name: APP_INFO.name, extensions: ['devnotes', 'json'] };
const SAVE_FILTER = { name: APP_INFO.name, extensions: ['devnotes'] };

/** `null` covers both a cancellation and the plugin being unavailable. */
@Injectable({ providedIn: 'root' })
export class FileDialogService {
  private readonly adapter = inject(FILE_DIALOG_ADAPTER);

  async pickBundle(): Promise<string | null> {
    return this.pick({ multiple: false, filters: [OPEN_FILTER] });
  }

  async pickAttachment(): Promise<string | null> {
    return this.pick({ multiple: false });
  }

  async chooseBundleDestination(defaultPath: string): Promise<string | null> {
    return this.destination({ defaultPath, filters: [SAVE_FILTER] });
  }

  /** No filter: an attachment can be of any type. */
  async chooseDestination(defaultPath: string): Promise<string | null> {
    return this.destination({ defaultPath });
  }

  private async destination(options: SaveDialogOptions): Promise<string | null> {
    try {
      return await this.adapter.save(options);
    } catch {
      return null;
    }
  }

  /** `multiple: false` is asked for, but the plugin's return type stays a union. */
  private async pick(options: OpenDialogOptions): Promise<string | null> {
    try {
      const chosen = await this.adapter.open(options);
      if (Array.isArray(chosen)) return chosen[0] ?? null;
      return chosen;
    } catch {
      return null;
    }
  }
}
