import { FileDialogAdapter } from '@core/services/dialogs/file-dialog.service';
import type { OpenDialogOptions, SaveDialogOptions } from '@tauri-apps/plugin-dialog';

/** `null` by default: a spec that forgets to arm a path must not silently import a file. */
export class FakeFileDialog implements FileDialogAdapter {
  /** Path the next `open()` resolves to; `null` stands for a cancelled dialog. */
  openPath: string | string[] | null = null;
  savePath: string | null = null;

  /** When set, the call throws the way the plugin does outside Tauri. */
  throwOnOpen: Error | null = null;
  throwOnSave: Error | null = null;

  openCalls: OpenDialogOptions[] = [];
  saveCalls: SaveDialogOptions[] = [];

  async open(options: OpenDialogOptions): Promise<string | string[] | null> {
    this.openCalls.push(options);
    if (this.throwOnOpen) throw this.throwOnOpen;
    return this.openPath;
  }

  async save(options: SaveDialogOptions): Promise<string | null> {
    this.saveCalls.push(options);
    if (this.throwOnSave) throw this.throwOnSave;
    return this.savePath;
  }
}
