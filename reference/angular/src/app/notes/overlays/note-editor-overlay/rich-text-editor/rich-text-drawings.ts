import Image from '@tiptap/extension-image';
import type { Node as ProseMirrorNode } from '@tiptap/pm/model';

/** How a Note names a drawing it embeds: an image whose source is the drawing's id. */
export const DRAWING_SOURCE = 'devnotes:drawing/';

export interface EmbedHooks {
  /** The drawing's picture, opened only now: the Note holds its id alone. */
  picture(id: string): Promise<string | null>;
  /** What stands where a drawing was deleted. */
  missing(): string;
}

export function drawingSource(id: string): string {
  return `${DRAWING_SOURCE}${id}`;
}

/**
 * Markdown images, of which a Note draws only the drawings it embeds: their picture is asked
 * for by id, so the Note's body never carries one. Any other image stays as it was written —
 * the CSP lets no address but the app's be loaded, and a Note never fetches one.
 */
export const DrawingEmbeds = (hooks: EmbedHooks) =>
  Image.extend({
    addNodeView() {
      return ({ node }) => {
        const dom = document.createElement('span');
        dom.className = 'rich-embed';
        dom.contentEditable = 'false';
        dom.dataset['testid'] = 'rich-embed';
        let shown = 0;

        const show = (current: ProseMirrorNode): void => {
          const ask = ++shown;
          const source = String(current.attrs['src'] ?? '');
          const alt = String(current.attrs['alt'] ?? '');
          if (!source.startsWith(DRAWING_SOURCE)) {
            dom.classList.add('is-other');
            dom.textContent = `![${alt}](${source})`;
            return;
          }
          void hooks.picture(source.slice(DRAWING_SOURCE.length)).then((picture) => {
            if (ask !== shown) return;
            if (picture) {
              const image = document.createElement('img');
              image.src = picture;
              image.alt = alt;
              dom.classList.remove('is-missing');
              dom.replaceChildren(image);
            } else {
              dom.classList.add('is-missing');
              dom.textContent = `${hooks.missing()} · ${alt}`;
            }
          });
        };

        show(node);
        return {
          dom,
          update: (updated) => {
            if (updated.type !== node.type) return false;
            show(updated);
            return true;
          },
        };
      };
    },
  }).configure({ inline: true });
