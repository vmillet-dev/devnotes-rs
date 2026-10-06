import { ChangeDetectionStrategy, Component, OnInit, computed, inject, resource } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { HttpRepository } from '@core/data/http.repository';
import { requestBadge } from '@core/model/http.model';
import { HttpCollectionsStore } from '@core/services/http/http-collections.store';
import { SettingsStore } from '@core/services/settings/settings.store';
import { HttpRailComponent } from './rail/http-rail.component';

/**
 * The HTTP area: its rail of collections beside the open request. The rail is the library's,
 * and so is `Ctrl+B`; while it is hidden, the titlebar carries the switch between areas.
 */
@Component({
  selector: 'app-http-page',
  imports: [HttpRailComponent, TranslocoPipe],
  templateUrl: './http-page.component.html',
  styleUrl: './http-page.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
  host: { '(document:keydown)': 'onKeydown($event)' },
})
export class HttpPageComponent implements OnInit {
  protected readonly store = inject(HttpCollectionsStore);
  protected readonly settings = inject(SettingsStore);
  private readonly repository = inject(HttpRepository);

  private readonly requestResource = resource({
    params: () => this.store.openId() ?? undefined,
    loader: ({ params }) => this.repository.request(params),
  });

  protected readonly request = computed(() =>
    this.requestResource.hasValue() ? this.requestResource.value() : null,
  );
  protected readonly badge = computed(() => {
    const request = this.request();
    return request ? requestBadge(request) : null;
  });

  ngOnInit(): void {
    void this.store.load();
  }

  protected onKeydown(event: KeyboardEvent): void {
    const ctrlB =
      (event.ctrlKey || event.metaKey) && !event.altKey && !event.shiftKey && event.key.toLowerCase() === 'b';
    if (!ctrlB || event.defaultPrevented) return;
    event.preventDefault();
    this.settings.showLibraryRail.write(!this.settings.showLibraryRail());
  }
}
