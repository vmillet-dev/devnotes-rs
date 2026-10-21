import {
  ChangeDetectionStrategy,
  Component,
  DestroyRef,
  ElementRef,
  afterRenderEffect,
  computed,
  inject,
  input,
  output,
  signal,
  untracked,
  viewChild,
} from '@angular/core';
import { TranslocoPipe, TranslocoService } from '@jsverse/transloco';
import type { Editor } from '@tiptap/core';
import { Presentation } from '@core/model/note.model';
import { MermaidRenderer } from '@core/services/diagrams/mermaid-renderer';
import { DrawingPicturesStore } from '@core/state/drawing-pictures.store';
import { AppWindowService } from '@core/services/window/app-window.service';
import { DialogComponent } from '@shared/layout/dialog/dialog.component';
import { createRichViewer } from '../note-editor-overlay/rich-text-editor/rich-text.engine';
import { drawnHooks } from '../note-editor-overlay/rich-text-editor/drawn-hooks';
import { MindMapComponent } from './mind-map/mind-map.component';

/** What moves a slide, as a presenter's clicker sends it too. */
const NEXT = new Set(['ArrowRight', 'ArrowDown', 'PageDown', ' ', 'Enter']);
const PREVIOUS = new Set(['ArrowLeft', 'ArrowUp', 'PageUp', 'Backspace']);

/**
 * A Note full screen, a slide at a time: its title first, then each slide Rust cut from the
 * draft, drawn by the rich editor read only. `M` turns to the map of its headings, and a
 * heading pressed there opens its slide. Escape leaves, through the modal frame.
 *
 * The window is made full screen for as long as this lives, and given back as it was.
 */
@Component({
  selector: 'app-presentation',
  imports: [DialogComponent, MindMapComponent, TranslocoPipe],
  templateUrl: './presentation.component.html',
  styleUrl: './presentation.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
  host: {
    '(document:keydown)': 'onKeydown($event)',
  },
})
export class PresentationComponent {
  readonly title = input.required<string>();
  readonly presentation = input.required<Presentation>();

  readonly closed = output<void>();

  private readonly window = inject(AppWindowService);
  private readonly diagrams = inject(MermaidRenderer);
  private readonly drawings = inject(DrawingPicturesStore);
  private readonly transloco = inject(TranslocoService);
  private readonly surface = viewChild.required<ElementRef<HTMLElement>>('surface');

  /** 0 is the cover; the body's slides follow it. */
  protected readonly at = signal(0);
  protected readonly showingMap = signal(false);
  protected readonly count = computed(() => this.presentation().slides.length + 1);
  protected readonly progress = computed(() => ((this.at() + 1) / this.count()) * 100);

  /** Asked at once, and answered whether the window changed: only that is given back. */
  private readonly screen = this.window.enterFullscreen();
  private viewer: Editor | null = null;

  constructor() {
    afterRenderEffect(() => {
      const at = this.at();
      const markdown = this.presentation().slides[at - 1];
      if (markdown === undefined) return;
      untracked(() => this.draw(markdown));
    });

    inject(DestroyRef).onDestroy(() => {
      this.viewer?.destroy();
      void this.screen.then((took) => (took ? this.window.leaveFullscreen() : undefined));
    });
  }

  protected go(at: number): void {
    this.at.set(Math.min(Math.max(at, 0), this.count() - 1));
  }

  /** A heading of the map opens its slide; the title, the cover. */
  protected open(slide: number | null): void {
    this.showingMap.set(false);
    this.go(slide === null ? 0 : slide + 1);
  }

  protected toggleMap(): void {
    this.showingMap.update((showing) => !showing);
  }

  protected onKeydown(event: KeyboardEvent): void {
    if (event.ctrlKey || event.metaKey || event.altKey) return;
    // A button that has the focus answers Enter and Space itself.
    if ((event.key === 'Enter' || event.key === ' ') && event.target instanceof HTMLButtonElement) return;

    if (event.key.toLowerCase() === 'm') {
      this.toggleMap();
    } else if (this.showingMap()) {
      return;
    } else if (NEXT.has(event.key)) {
      this.go(this.at() + 1);
    } else if (PREVIOUS.has(event.key)) {
      this.go(this.at() - 1);
    } else if (event.key === 'Home') {
      this.go(0);
    } else if (event.key === 'End') {
      this.go(this.count() - 1);
    } else {
      return;
    }
    event.preventDefault();
  }

  private draw(markdown: string): void {
    if (this.viewer) {
      this.viewer.commands.setContent(markdown, { contentType: 'markdown', emitUpdate: false });
      return;
    }
    this.viewer = createRichViewer(
      this.surface().nativeElement,
      markdown,
      drawnHooks(this.diagrams, this.drawings, this.transloco, false),
    );
  }
}
