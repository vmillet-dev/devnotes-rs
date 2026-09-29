import {
  ChangeDetectionStrategy,
  Component,
  ElementRef,
  Injector,
  afterNextRender,
  computed,
  inject,
  input,
  output,
  viewChild,
} from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { JsonLine } from '@core/model/json.model';

/**
 * The same document as an indented tree: the view for a large one, a line per value. Its
 * lines are Rust's, open or not by the same set as the graph's nodes.
 */
@Component({
  selector: 'app-json-tree',
  imports: [TranslocoPipe],
  templateUrl: './json-tree.component.html',
  styleUrl: './json-tree.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class JsonTreeComponent {
  private readonly injector = inject(Injector);

  readonly lines = input.required<readonly JsonLine[]>();
  readonly truncated = input(false);
  readonly selectedPath = input<string | null>(null);

  readonly selected = output<string>();
  /** A container's line: it opens or closes. */
  readonly toggled = output<JsonLine>();

  private readonly list = viewChild.required<ElementRef<HTMLElement>>('list');

  protected readonly cursor = computed(() => {
    const index = this.lines().findIndex((line) => line.path === this.selectedPath());
    return index < 0 ? 0 : index;
  });

  protected choose(line: JsonLine): void {
    if (line.opens) this.toggled.emit(line);
    else this.selected.emit(line.path);
  }

  protected activate(line: JsonLine, event: Event): void {
    event.preventDefault();
    this.choose(line);
    this.refocus();
  }

  protected onKeydown(event: KeyboardEvent): void {
    const lines = this.lines();
    const index = this.cursor();
    const line = lines[index];
    if (!line) return;

    switch (event.key) {
      case 'ArrowDown':
        this.moveTo(Math.min(lines.length - 1, index + 1), event);
        break;
      case 'ArrowUp':
        this.moveTo(Math.max(0, index - 1), event);
        break;
      case 'ArrowRight':
        if (line.opens && !line.open) this.toggle(line, event);
        else if (line.open) this.moveTo(index + 1, event);
        break;
      case 'ArrowLeft':
        if (line.open && index > 0) this.toggle(line, event);
        else this.moveTo(this.parentOf(index), event);
        break;
    }
  }

  private parentOf(index: number): number {
    const depth = this.lines()[index]!.depth;
    for (let at = index - 1; at >= 0; at--) {
      if (this.lines()[at]!.depth < depth) return at;
    }
    return index;
  }

  private moveTo(index: number, event: KeyboardEvent): void {
    const line = this.lines()[index];
    if (!line) return;
    event.preventDefault();
    this.selected.emit(line.path);
    this.refocus();
  }

  private toggle(line: JsonLine, event: KeyboardEvent): void {
    event.preventDefault();
    this.toggled.emit(line);
    this.refocus();
  }

  private refocus(): void {
    afterNextRender(
      () => {
        const current = this.list().nativeElement.querySelector<HTMLElement>('[tabindex="0"]');
        current?.focus();
        current?.scrollIntoView({ block: 'nearest' });
      },
      { injector: this.injector },
    );
  }
}
