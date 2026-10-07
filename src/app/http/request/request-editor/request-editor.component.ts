import { ChangeDetectionStrategy, Component, computed, inject, input, output, signal } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { HTTP_METHODS, HttpMethod, KeyValueRow, requestBadge } from '@core/model/http.model';
import { HttpTabsStore, RequestTab } from '@core/services/http/http-tabs.store';
import { ChoiceMenuComponent, ChoiceOption } from '@shared/controls/choice-menu/choice-menu.component';
import { HTTP_HEADERS } from '@tools/catalogue/http-headers/http-headers.data';
import { KeyValueTableComponent } from '../key-value-table/key-value-table.component';
import { UrlFieldComponent } from '../url-field/url-field.component';

type Section = 'params' | 'headers';

/** A header a request may send: the reference's own list, those a server alone writes left out. */
const REQUEST_HEADERS = HTTP_HEADERS.filter((header) => header.direction !== 'response').map(
  (header) => header.name,
);

const counted = (rows: readonly KeyValueRow[]) => rows.filter((row) => row.enabled && row.key !== '').length;

/** The open request: its method, its URL, and its parts a tab each. */
@Component({
  selector: 'app-request-editor',
  imports: [ChoiceMenuComponent, KeyValueTableComponent, TranslocoPipe, UrlFieldComponent],
  templateUrl: './request-editor.component.html',
  styleUrl: './request-editor.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class RequestEditorComponent {
  readonly tab = input.required<RequestTab>();
  /** A request never saved asks where to go first: the page owns that dialog. */
  readonly saveAsked = output<void>();

  protected readonly tabs = inject(HttpTabsStore);

  protected readonly section = signal<Section>('params');
  protected readonly sections: readonly Section[] = ['params', 'headers'];
  protected readonly headerNames = REQUEST_HEADERS;

  protected readonly methods: readonly ChoiceOption[] = HTTP_METHODS.map((method) => ({
    id: method,
    name: method,
  }));

  protected readonly badge = computed(() => requestBadge(this.tab().draft));
  protected readonly dirty = computed(() => this.tabs.isDirty(this.tab()));
  protected readonly counts = computed<Record<Section, number>>(() => ({
    params: counted(this.tab().draft.parts.params),
    headers: counted(this.tab().draft.parts.headers),
  }));

  protected onMethod(method: string | null): void {
    if (method !== null) this.tabs.edit(this.tab().key, { method: method as HttpMethod });
  }

  protected onName(event: Event): void {
    this.tabs.edit(this.tab().key, { name: (event.target as HTMLInputElement).value });
  }

  protected onUrl(url: string): void {
    void this.tabs.editUrl(this.tab().key, url);
  }

  protected onParams(params: readonly KeyValueRow[]): void {
    void this.tabs.editParams(this.tab().key, params);
  }

  protected onHeaders(headers: readonly KeyValueRow[]): void {
    this.tabs.editParts(this.tab().key, { headers });
  }

  protected save(): void {
    if (this.tab().requestId === null) this.saveAsked.emit();
    else void this.tabs.save(this.tab().key);
  }
}
