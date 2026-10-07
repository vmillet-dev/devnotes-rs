import { ChangeDetectionStrategy, Component, computed, input, output } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { Transport, TransportSettings } from '@core/model/http.model';
import { ChoiceMenuComponent, ChoiceOption } from '@shared/controls/choice-menu/choice-menu.component';

type Switch = 'followRedirects' | 'verifyTls' | 'useCookies';
type Count = 'timeoutMs' | 'maxRedirects';

const SWITCHES: readonly Switch[] = ['followRedirects', 'verifyTls', 'useCookies'];
const SHORTEST: Record<Count, number> = { timeoutMs: 100, maxRedirects: 0 };

/** How a request travels; each field left unset takes what is above it, shown beside it. */
@Component({
  selector: 'app-transport-editor',
  imports: [ChoiceMenuComponent, TranslocoPipe],
  templateUrl: './transport-editor.component.html',
  styleUrl: './transport-editor.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class TransportEditorComponent {
  readonly settings = input.required<TransportSettings>();
  /** What an unset field takes: the folders and the collection above, or the defaults. */
  readonly above = input.required<Transport>();
  /** A collection has nothing above it: its unset fields are the defaults. */
  readonly aboveIsDefault = input(false);
  readonly settingsChange = output<TransportSettings>();

  protected readonly switches = SWITCHES;
  protected readonly inheritKey = computed(() =>
    this.aboveIsDefault() ? 'http.transport.byDefault' : 'http.transport.inherited',
  );

  protected readonly insecure = computed(() => !(this.settings().verifyTls ?? this.above().verifyTls));

  protected readonly options = computed<readonly ChoiceOption[]>(() => [
    { id: 'inherit', name: this.inheritKey(), nameIsKey: true },
    { id: 'on', name: 'http.transport.on', nameIsKey: true },
    { id: 'off', name: 'http.transport.off', nameIsKey: true },
  ]);

  protected owns(field: keyof TransportSettings): boolean {
    const own = this.settings()[field];
    return own !== undefined && own !== null;
  }

  protected switchId(field: Switch): string {
    if (!this.owns(field)) return 'inherit';
    return this.settings()[field] ? 'on' : 'off';
  }

  protected onSwitch(field: Switch, id: string | null): void {
    const value = id === 'on' ? true : id === 'off' ? false : null;
    this.settingsChange.emit({ ...this.settings(), [field]: value });
  }

  protected onNumber(field: Count, event: Event): void {
    const typed = (event.target as HTMLInputElement).value.trim();
    const parsed = Number.parseInt(typed, 10);
    const value = typed === '' || Number.isNaN(parsed) ? null : Math.max(parsed, SHORTEST[field]);
    this.settingsChange.emit({ ...this.settings(), [field]: value });
  }
}
