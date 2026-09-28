import { ChangeDetectionStrategy, Component, input } from '@angular/core';
import { OutlineLine } from '@core/model/note.model';

/** A Note on its card: its shape — headings, items, code — read by Rust from the Markdown. */
@Component({
  selector: 'app-note-outline',
  templateUrl: './note-outline.component.html',
  styleUrl: './note-outline.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class NoteOutlineComponent {
  readonly lines = input.required<readonly OutlineLine[]>();
}
