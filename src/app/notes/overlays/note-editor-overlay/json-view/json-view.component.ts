import { ChangeDetectionStrategy, Component, inject, input, output } from '@angular/core';
import { Span } from '@core/model/json.model';
import { ClipboardService } from '@core/services/clipboard/clipboard.service';
import { LocaleService } from '@core/services/i18n/locale.service';
import { JsonExplorerStore } from '@core/state/json-explorer.store';
import { JsonExplorerComponent, JsonExplorerMode } from '@shared/json-explorer/json-explorer.component';

/**
 * The editor's side of the visualiser: the store the editor provides, the clipboard and the
 * language, handed to an explorer that takes inputs only.
 */
@Component({
  selector: 'app-json-view',
  imports: [JsonExplorerComponent],
  templateUrl: './json-view.component.html',
  styleUrl: './json-view.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class JsonViewComponent {
  protected readonly store = inject(JsonExplorerStore);
  protected readonly locale = inject(LocaleService);
  private readonly clipboard = inject(ClipboardService);

  readonly mode = input.required<JsonExplorerMode>();

  readonly shownInCode = output<Span>();
  readonly modeChanged = output<JsonExplorerMode>();

  protected copy(text: string): void {
    void this.clipboard.copy(text);
  }
}
