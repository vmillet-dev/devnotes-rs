import {
  ChangeDetectionStrategy,
  Component,
  DestroyRef,
  ElementRef,
  afterNextRender,
  effect,
  inject,
  input,
  output,
  signal,
  untracked,
  viewChild,
} from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import type { DrawingPictures } from '@core/model/note.model';
import { DrawingPicturesStore } from '@core/state/drawing-pictures.store';
import { DrawingExportService, DrawingFormat } from '@core/services/drawing/drawing-export.service';
import { HUES, Hue, Tool } from './drawing-geometry';
import { DrawingScene, paletteOf, sceneOf } from './drawing-scene';

interface ToolEntry {
  readonly id: Tool;
  readonly glyph: string;
  readonly key: string;
}

const TOOLS: readonly ToolEntry[] = [
  { id: 'select', glyph: '↖', key: 'v' },
  { id: 'rectangle', glyph: '▭', key: 'r' },
  { id: 'ellipse', glyph: '◯', key: 'o' },
  { id: 'diamond', glyph: '◇', key: 'd' },
  { id: 'arrow', glyph: '→', key: 'a' },
  { id: 'pen', glyph: '✎', key: 'p' },
  { id: 'text', glyph: 'T', key: 't' },
];

/** What is drawn is saved once the hand pauses: each save takes two pictures of it. */
export const SAVE_AFTER_MS = 400;

/**
 * A Dessin's editor: Fabric's canvas under the app's toolbar. The scene is the body, emitted
 * once drawing pauses; `flush`, which every closing path and the blur call, emits what is
 * pending and the two pictures Rust keeps beside it, for a card and for a Note.
 */
@Component({
  selector: 'app-drawing-editor',
  imports: [TranslocoPipe],
  templateUrl: './drawing-editor.component.html',
  styleUrl: './drawing-editor.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
  host: {
    '(keydown)': 'onKeydown($event)',
    '(focusout)': 'onFocusOut($event)',
  },
})
export class DrawingEditorComponent {
  readonly content = input.required<string>();
  readonly title = input('');
  /** Whose pictures are taken when it has none yet, as a drawing that came by import. */
  readonly noteId = input<string | null>(null);

  readonly changed = output<string>();
  /** Taken with the body that `flush` emits, not as it is drawn: two pictures are dear. */
  readonly pictured = output<DrawingPictures>();
  readonly blurred = output<void>();
  /** How many shapes it holds, for the footer. */
  readonly counted = output<number>();

  private readonly exports = inject(DrawingExportService);
  private readonly drawings = inject(DrawingPicturesStore);
  private readonly host = inject<ElementRef<HTMLElement>>(ElementRef);
  private readonly canvasElement = viewChild.required<ElementRef<HTMLCanvasElement>>('canvas');
  private readonly stage = viewChild.required<ElementRef<HTMLElement>>('stage');

  protected readonly tools = TOOLS;
  protected readonly hues = HUES;
  protected readonly tool = signal<Tool>('select');
  protected readonly hue = signal<Hue>('ink');
  protected readonly shapes = signal(0);
  protected readonly empty = signal(true);
  protected readonly canUndo = signal(false);
  protected readonly canRedo = signal(false);

  private scene: DrawingScene | null = null;
  /** The body last emitted or loaded: a `content` that is not it is another drawing. */
  private written = '';
  private saving: ReturnType<typeof setTimeout> | undefined;
  /** Something was drawn since the pictures were last taken. */
  private unpictured = false;
  private stopObserving: (() => void) | null = null;

  constructor() {
    afterNextRender(() => void this.open());

    effect(() => {
      const content = this.content();
      untracked(() => {
        if (this.scene && content !== this.written) void this.load(content);
      });
    });

    inject(DestroyRef).onDestroy(() => {
      this.flush();
      this.stopObserving?.();
      this.scene?.dispose();
    });
  }

  /** What is drawn and not yet saved goes now, its pictures with it. */
  flush(): void {
    if (this.saving !== undefined) {
      clearTimeout(this.saving);
      this.saving = undefined;
      this.emit();
    }
    if (this.unpictured && this.scene) {
      this.unpictured = false;
      this.pictured.emit(this.scene.pictures());
    }
  }

  protected choose(tool: Tool): void {
    this.tool.set(tool);
    this.scene?.setTool(tool);
    this.stage().nativeElement.focus();
  }

  protected pick(hue: Hue): void {
    this.hue.set(hue);
    this.scene?.setHue(hue);
  }

  protected async undo(): Promise<void> {
    await this.scene?.undo();
  }

  protected async redo(): Promise<void> {
    await this.scene?.redo();
  }

  protected remove(): void {
    this.scene?.removeSelection();
  }

  protected async export(format: DrawingFormat): Promise<void> {
    const picture = format === 'png' ? this.scene?.png() : this.scene?.svg();
    if (picture) await this.exports.save(this.title(), format, picture);
  }

  /**
   * The tools by their letter, the history by its keys; Escape first drops the tool, then the
   * selection, and only then leaves, to the modal frame.
   */
  protected onKeydown(event: KeyboardEvent): void {
    const scene = this.scene;
    if (!scene || scene.editingText()) return;
    const key = event.key.toLowerCase();
    const command = event.ctrlKey || event.metaKey;

    if (command && key === 'z') {
      void (event.shiftKey ? this.redo() : this.undo());
    } else if (command && key === 'y') {
      void this.redo();
    } else if (command || event.altKey) {
      return;
    } else if (event.key === 'Delete' || event.key === 'Backspace') {
      if (!scene.removeSelection()) return;
    } else if (event.key === 'Escape') {
      if (this.tool() !== 'select') {
        this.tool.set('select');
        scene.backToSelection();
      } else if (!scene.deselect()) {
        return;
      }
      event.stopPropagation();
    } else {
      const tool = TOOLS.find((entry) => entry.key === key);
      if (!tool) return;
      this.choose(tool.id);
    }
    event.preventDefault();
  }

  /** Leaving the drawing for the rest of the editor saves it, as the code field's blur does. */
  protected onFocusOut(event: FocusEvent): void {
    const next = event.relatedTarget;
    if (next instanceof Node && this.host.nativeElement.contains(next)) return;
    this.flush();
    this.blurred.emit();
  }

  private async open(): Promise<void> {
    const stage = this.stage().nativeElement;
    this.scene = new DrawingScene(
      this.canvasElement().nativeElement,
      paletteOf(getComputedStyle(document.documentElement)),
      {
        changed: () => this.onChanged(),
        made: () => this.tool.set('select'),
      },
    );
    this.fit(stage);
    await this.load(this.content());

    const resizing = typeof ResizeObserver === 'undefined' ? null : new ResizeObserver(() => this.fit(stage));
    resizing?.observe(stage);
    // A theme changed in the settings: what is drawn takes its colours.
    const theming = new MutationObserver(() =>
      this.scene?.recolour(paletteOf(getComputedStyle(document.documentElement))),
    );
    theming.observe(document.documentElement, { attributes: true, attributeFilter: ['data-theme'] });
    this.stopObserving = () => {
      resizing?.disconnect();
      theming.disconnect();
    };
  }

  private async load(content: string): Promise<void> {
    this.written = content;
    await this.scene?.load(sceneOf(content));
    this.reflect();
    const id = this.noteId();
    if (id && this.scene?.shapes() && !(await this.drawings.hasPictures(id))) this.unpictured = true;
  }

  private fit(stage: HTMLElement): void {
    this.scene?.resize(stage.clientWidth, stage.clientHeight);
  }

  private onChanged(): void {
    this.reflect();
    this.unpictured = true;
    clearTimeout(this.saving);
    this.saving = setTimeout(() => {
      this.saving = undefined;
      this.emit();
    }, SAVE_AFTER_MS);
  }

  private reflect(): void {
    const scene = this.scene;
    if (!scene) return;
    this.shapes.set(scene.shapes());
    this.empty.set(scene.shapes() === 0);
    this.canUndo.set(scene.canUndo());
    this.canRedo.set(scene.canRedo());
    this.counted.emit(scene.shapes());
  }

  private emit(): void {
    const scene = this.scene;
    if (!scene) return;
    this.written = scene.body();
    this.changed.emit(this.written);
  }
}
