import { Directive, inject } from '@angular/core';
import { areaForKey, isToolSearchKey } from '@core/services/areas/area.model';
import { AreaStore } from '@core/services/areas/area.store';
import { ToolsStore } from '@core/services/tools/tools.store';
import { VaultStore } from '@core/state/vault.store';
import { DialogStack } from '@shared/layout/dialog/dialog-stack';

/** From any field too: Ctrl and a digit types nothing anywhere. Never behind a modal. */
@Directive({
  selector: '[appAreaKeys]',
  host: { '(document:keydown)': 'onKeydown($event)' },
})
export class AreaKeysDirective {
  private readonly areas = inject(AreaStore);
  private readonly vault = inject(VaultStore);
  private readonly dialogs = inject(DialogStack);
  private readonly tools = inject(ToolsStore);

  protected onKeydown(event: KeyboardEvent): void {
    if (!this.vault.isUnlocked() || this.dialogs.hasOpenDialog() || event.defaultPrevented) return;

    if (isToolSearchKey(event)) {
      event.preventDefault();
      this.areas.show('tools');
      this.tools.requestSearch();
      return;
    }

    const area = areaForKey(event);
    if (area === null) return;

    event.preventDefault();
    this.areas.show(area);
  }
}
