import { ChangeDetectionStrategy, Component, inject, input, output } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { Folder } from '@core/model/folder.model';
import { FolderEditorComponent } from '@notes/header/folder-editor/folder-editor.component';
import { MenuPanelDirective } from '@shared/directives/menu-panel.directive';
import { MenuTriggerDirective } from '@shared/directives/menu-trigger.directive';

/**
 * `SQL / ● Perf`, and the way back out. It replaces the space switcher and the view
 * switch while a folder is open: there is one place to go from here, and it is back.
 */
@Component({
  selector: 'app-folder-breadcrumb',
  imports: [TranslocoPipe, MenuPanelDirective, FolderEditorComponent],
  hostDirectives: [MenuTriggerDirective],
  templateUrl: './folder-breadcrumb.component.html',
  styleUrl: './folder-breadcrumb.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class FolderBreadcrumbComponent {
  readonly folder = input.required<Folder>();
  /** `null` while the user is on every space, which the breadcrumb then leaves out. */
  readonly spaceName = input<string | null>(null);
  /** How many of the folder's notes are on screen; the whole view is its contents. */
  readonly selectableCount = input<number | null>(null);

  readonly closed = output<void>();
  readonly selectRequested = output<string>();

  protected readonly menu = inject(MenuTriggerDirective);

  constructor() {
    this.menu.escaped.subscribe(() => this.menu.close());
  }

  protected onSelectRequested(folderId: string): void {
    this.selectRequested.emit(folderId);
    this.menu.close();
  }
}
