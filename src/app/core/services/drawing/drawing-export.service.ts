import { Injectable, inject } from '@angular/core';
import { ToolsRepository } from '@core/data/tools.repository';
import { FileDialogService } from '@core/services/dialogs/file-dialog.service';
import { ErrorNotifier } from '@core/services/errors/error-notifier.service';
import { StatusNotifier } from '@core/services/notifications/status.service';

export type DrawingFormat = 'png' | 'svg';

/** What a file name cannot hold on any system the app runs on. */
const FORBIDDEN = /[\\/:*?"<>|]/g;

/**
 * A drawing written where the user says: a PNG as the scene drew it, an SVG as its text. Both
 * cross as a data URI, which Rust decodes and writes as the base64 tool's bytes are.
 */
@Injectable({ providedIn: 'root' })
export class DrawingExportService {
  private readonly files = inject(FileDialogService);
  private readonly tools = inject(ToolsRepository);
  private readonly notifier = inject(ErrorNotifier);
  private readonly status = inject(StatusNotifier);

  async save(title: string, format: DrawingFormat, picture: string): Promise<void> {
    const path = await this.files.chooseImageDestination(`${fileNameOf(title)}.${format}`, format);
    if (path === null) return;

    const uri = format === 'png' ? picture : `data:image/svg+xml;base64,${base64Of(picture)}`;
    const saved = await this.notifier.attempt('drawing.exportFailed', () =>
      this.tools.saveBytes({ text: uri, reading: null }, path),
    );
    if (saved?.kind === 'saved') {
      this.status.notify({ key: 'drawing.exported', params: { name: baseNameOf(path) } });
    } else if (saved?.kind === 'failed') {
      this.notifier.notify({ ref: { key: `tools.files.${saved.problem}` } });
    }
  }
}

export function fileNameOf(title: string): string {
  const printable = [...title].filter((character) => character >= ' ').join('');
  return printable.replace(FORBIDDEN, '-').trim() || 'dessin';
}

function baseNameOf(path: string): string {
  return path.split(/[\\/]/).pop() ?? path;
}

/** UTF-8 first: `btoa` reads one byte a character. In slices, for a long text's sake. */
export function base64Of(text: string): string {
  const bytes = new TextEncoder().encode(text);
  let binary = '';
  for (let at = 0; at < bytes.length; at += 0x8000) {
    binary += String.fromCharCode(...bytes.subarray(at, at + 0x8000));
  }
  return btoa(binary);
}
