import { ChangeDetectionStrategy, Component, inject, input, output } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { NoteKind } from '@core/model/note.model';
import { MenuPanelDirective } from '@shared/directives/menu-panel.directive';
import { MenuTriggerDirective } from '@shared/directives/menu-trigger.directive';

interface KindEntry {
  readonly id: NoteKind;
  readonly glyph: string;
  readonly nameKey: string;
  readonly lineKey: string;
}

const KINDS: readonly KindEntry[] = [
  { id: 'snippet', glyph: '</>', nameKey: 'notes.newSnippet', lineKey: 'notes.kindLines.snippet' },
  { id: 'note', glyph: 'Aa', nameKey: 'notes.newRichNote', lineKey: 'notes.kindLines.note' },
  { id: 'checklist', glyph: '✓', nameKey: 'notes.newChecklist', lineKey: 'notes.kindLines.checklist' },
];

/** A split button: creating a note keeps one click, picking a kind takes two. */
@Component({
  selector: 'app-new-note-button',
  imports: [TranslocoPipe, MenuPanelDirective],
  hostDirectives: [MenuTriggerDirective],
  templateUrl: './new-note-button.component.html',
  styleUrl: './new-note-button.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class NewNoteButtonComponent {
  /** The canvas keys each kind answers to, as bound now. */
  readonly shortcuts = input.required<Readonly<Record<NoteKind, readonly string[]>>>();
  readonly noteIsNew = input(false);

  readonly created = output<NoteKind>();

  protected readonly menu = inject(MenuTriggerDirective);

  protected readonly kinds = KINDS;

  protected create(kind: NoteKind): void {
    this.created.emit(kind);
    this.menu.close();
  }

  /** ARIA spells the modifier out: `Control`, not the caps' `Ctrl`. */
  protected ariaShortcut(kind: NoteKind): string {
    const keys = this.shortcuts()[kind];

    return keys.map((key) => (key === 'Ctrl' ? 'Control' : key)).join('+');
  }
}
