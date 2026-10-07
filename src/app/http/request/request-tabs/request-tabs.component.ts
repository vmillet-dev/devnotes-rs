import { ChangeDetectionStrategy, Component, inject, output } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { requestBadge } from '@core/model/http.model';
import { HttpTabsStore, RequestTab } from '@core/services/http/http-tabs.store';

/** One tab an open request: its method and name, a dot while modified, a × to close. */
@Component({
  selector: 'app-request-tabs',
  imports: [TranslocoPipe],
  templateUrl: './request-tabs.component.html',
  styleUrl: './request-tabs.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class RequestTabsComponent {
  protected readonly tabs = inject(HttpTabsStore);

  /** A modified tab is not closed here: the page asks first. */
  readonly closeAsked = output<RequestTab>();

  protected badge(tab: RequestTab): string {
    return requestBadge(tab.draft);
  }

  protected close(tab: RequestTab): void {
    if (this.tabs.isDirty(tab)) this.closeAsked.emit(tab);
    else this.tabs.close(tab.key);
  }

  /** The middle button closes a tab, as in a browser. */
  protected onAuxClick(event: MouseEvent, tab: RequestTab): void {
    if (event.button !== 1) return;
    event.preventDefault();
    this.close(tab);
  }
}
