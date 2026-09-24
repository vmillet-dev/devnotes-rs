import { ChangeDetectionStrategy, Component, computed, inject, input, output } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { BoardScope } from '@core/model/board.model';
import { MenuPanelDirective } from '@shared/directives/menu-panel.directive';
import { MenuTriggerDirective } from '@shared/directives/menu-trigger.directive';

/**
 * A split button, like the new-note one: aligning the loose cards keeps one click, and
 * reorganising everything takes two.
 *
 * ⚠️ The split is the point, not decoration. The cards outside the zones are what goes to
 * pieces, and they cost nothing to redo; a zone somebody positioned and sized by hand is
 * the only manual work a board holds. One control did both, so the click that repaired the
 * cheap half destroyed the expensive one — which is what stops anyone pressing it twice.
 */
@Component({
  selector: 'app-board-tidy',
  imports: [TranslocoPipe, MenuPanelDirective],
  hostDirectives: [MenuTriggerDirective],
  templateUrl: './board-tidy.component.html',
  styleUrl: './board-tidy.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class BoardTidyComponent {
  readonly zoneCount = input.required<number>();
  readonly looseCount = input.required<number>();

  readonly requested = output<BoardScope>();

  protected readonly menu = inject(MenuTriggerDirective);

  /**
   * ⚠️ The counts, not a warning. The project names what a corpus-wide action will touch
   * before it touches it — the tag manager and emptying the trash both do — and the numbers
   * are already in hand here, drawn from the view the board is showing. A ghost preview on
   * hover was refused: that is a second layout engine on the front for a tooltip.
   */
  protected readonly counts = computed(() => ({ zones: this.zoneCount(), cards: this.looseCount() }));

  protected ask(scope: BoardScope): void {
    this.requested.emit(scope);
    this.menu.close();
  }
}
