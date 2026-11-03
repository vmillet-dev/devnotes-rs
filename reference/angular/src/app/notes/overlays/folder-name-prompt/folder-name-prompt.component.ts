import {
  ChangeDetectionStrategy,
  Component,
  ElementRef,
  afterNextRender,
  output,
  viewChild,
} from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { DialogComponent } from '@shared/layout/dialog/dialog.component';

/**
 * What a band drawn on the board is called. The colour is not asked for: it is assigned
 * from the palette and changed from the zone menu, so drawing a folder stays one gesture.
 */
@Component({
  selector: 'app-folder-name-prompt',
  imports: [DialogComponent, TranslocoPipe],
  templateUrl: './folder-name-prompt.component.html',
  styleUrl: './folder-name-prompt.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class FolderNamePromptComponent {
  readonly submitted = output<string>();
  readonly cancelled = output<void>();

  private readonly nameInput = viewChild<ElementRef<HTMLInputElement>>('nameInput');

  constructor() {
    // The band is already drawn: the only thing left to do is type.
    afterNextRender(() => this.nameInput()?.nativeElement.focus());
  }

  protected submit(event: Event, name: string): void {
    event.preventDefault();
    if (!name.trim()) return;

    this.submitted.emit(name);
  }
}
