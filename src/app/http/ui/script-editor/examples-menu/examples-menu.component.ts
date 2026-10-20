import { ChangeDetectionStrategy, Component, inject, output } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { FloatingPanelDirective } from '@shared/directives/floating-panel.directive';
import { MenuPanelDirective } from '@shared/directives/menu-panel.directive';
import { MenuTriggerDirective } from '@shared/directives/menu-trigger.directive';

export type ScriptExample = 'readResponse' | 'setVariable' | 'addHeader' | 'sendRequest' | 'testStatus';

/** Five, so the menu stays short; each says the API it shows. */
export const SCRIPT_EXAMPLES: readonly { readonly id: ScriptExample; readonly api: string }[] = [
  { id: 'readResponse', api: 'res.json()' },
  { id: 'setVariable', api: 'env.set' },
  { id: 'addHeader', api: 'req.headers' },
  { id: 'sendRequest', api: 'pm.sendRequest' },
  { id: 'testStatus', api: 'test' },
];

/** « Exemples ▾ »: a block ready to adapt, written at the end of the script. */
@Component({
  selector: 'app-script-examples-menu',
  imports: [FloatingPanelDirective, MenuPanelDirective, TranslocoPipe],
  templateUrl: './examples-menu.component.html',
  styleUrl: './examples-menu.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
  hostDirectives: [MenuTriggerDirective],
})
export class ScriptExamplesMenuComponent {
  readonly chosen = output<ScriptExample>();

  protected readonly menu = inject(MenuTriggerDirective);
  protected readonly examples = SCRIPT_EXAMPLES;

  protected choose(example: ScriptExample): void {
    // Focus goes to the code the block was written into, not back to the button.
    this.menu.close(false);
    this.chosen.emit(example);
  }
}
