import { ChangeDetectionStrategy, Component, inject, output } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { requestBadge, shortBadge } from '@core/model/http.model';
import { HttpSendStore } from '@core/services/http/http-send.store';
import { HttpSocketsStore } from '@core/services/http/http-sockets.store';
import { HttpTabsStore, RequestTab } from '@core/services/http/http-tabs.store';
import { SettingsStore } from '@core/services/settings/settings.store';
import { RailToggleComponent } from '@shared/controls/rail-toggle/rail-toggle.component';
import { SpinnerComponent } from '@shared/spinner/spinner.component';

/**
 * One tab an open request: its method and name, a dot while modified, a × to close, and a
 * spinner while its send is under way — a request sent then left behind still says it runs.
 */
@Component({
  selector: 'app-request-tabs',
  imports: [RailToggleComponent, SpinnerComponent, TranslocoPipe],
  templateUrl: './request-tabs.component.html',
  styleUrl: './request-tabs.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class RequestTabsComponent {
  protected readonly tabs = inject(HttpTabsStore);
  protected readonly sending = inject(HttpSendStore);
  protected readonly settings = inject(SettingsStore);
  private readonly sockets = inject(HttpSocketsStore);

  /** A modified tab, or one whose socket is connected, is not closed here: the page asks first. */
  readonly closeAsked = output<RequestTab>();

  protected readonly short = shortBadge;

  protected badge(tab: RequestTab): string {
    return requestBadge(tab.draft);
  }

  protected close(tab: RequestTab): void {
    if (this.tabs.isDirty(tab) || this.sockets.isLive(tab.key)) this.closeAsked.emit(tab);
    else this.tabs.close(tab.key);
  }

  /** The middle button closes a tab, as in a browser. */
  protected onAuxClick(event: MouseEvent, tab: RequestTab): void {
    if (event.button !== 1) return;
    event.preventDefault();
    this.close(tab);
  }
}
