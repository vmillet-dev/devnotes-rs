import { ChangeDetectionStrategy, Component, inject, output } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { NoteKind } from '@core/model/note.model';
import { MenuPanelDirective } from '@shared/directives/menu-panel.directive';
import { MenuTriggerDirective } from '@shared/directives/menu-trigger.directive';

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
  readonly created = output<NoteKind>();

  protected readonly menu = inject(MenuTriggerDirective);

  protected create(kind: NoteKind): void {
    this.created.emit(kind);
    this.menu.close();
  }
}
