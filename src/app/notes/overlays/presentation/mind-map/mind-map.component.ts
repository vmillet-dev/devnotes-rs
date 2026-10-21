import { ChangeDetectionStrategy, Component, computed, input, output } from '@angular/core';
import { MapNode } from '@core/model/note.model';
import { layoutMindMap } from './mind-map.layout';

/**
 * The Note's headings as a map, read only: a heading pressed opens the slide it is on. The
 * headings are buttons over an SVG of their links, so each takes the focus and its ring.
 */
@Component({
  selector: 'app-mind-map',
  templateUrl: './mind-map.component.html',
  styleUrl: './mind-map.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class MindMapComponent {
  readonly title = input.required<string>();
  readonly map = input.required<readonly MapNode[]>();

  /** The body's slide of the heading pressed; `null` for the title, which is the cover. */
  readonly chosen = output<number | null>();

  protected readonly layout = computed(() => layoutMindMap(this.title(), this.map()));
}
