import type { TranslocoService } from '@jsverse/transloco';
import type { MermaidRenderer } from '@core/services/diagrams/mermaid-renderer';
import type { DrawingPicturesStore } from '@core/state/drawing-pictures.store';
import type { DrawnHooks } from './rich-text.engine';

/** What a Note's blocks are drawn with, in the editor and on a slide alike. */
export function drawnHooks(
  diagrams: MermaidRenderer,
  drawings: DrawingPicturesStore,
  transloco: TranslocoService,
  /** A slide is read only: its drawings do not offer to open their code. */
  editable: boolean,
): DrawnHooks {
  return {
    diagrams: {
      draw: (source) => diagrams.render(source),
      title: () => (editable ? transloco.translate('editor.rich.diagram.edit') : ''),
      failed: (line) =>
        line === null
          ? transloco.translate('editor.rich.diagram.failed')
          : transloco.translate('editor.rich.diagram.failedAt', { line }),
    },
    embeds: {
      picture: (id) => drawings.picture(id),
      missing: () => transloco.translate('drawing.missing'),
    },
  };
}
