import { ChangeDetectionStrategy, Component, OnInit, inject, signal } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { HttpCollectionsStore } from '@core/services/http/http-collections.store';
import { HttpSendStore } from '@core/services/http/http-send.store';
import { HttpTabsStore, RequestTab } from '@core/services/http/http-tabs.store';
import { SettingsStore } from '@core/services/settings/settings.store';
import { CloseChoice, CloseRequestDialogComponent } from './close-request/close-request-dialog.component';
import { HttpRailComponent } from './rail/http-rail.component';
import { RequestEditorComponent } from './request/request-editor/request-editor.component';
import { RequestTabsComponent } from './request/request-tabs/request-tabs.component';
import { ResponsePaneComponent } from './response/response-pane.component';
import { SaveAnswer, SaveRequestDialogComponent } from './save-request/save-request-dialog.component';

/**
 * The HTTP area: its rail of collections beside the open requests. The rail is the library's,
 * and so is `Ctrl+B`; while it is hidden, the titlebar carries the switch between areas.
 */
@Component({
  selector: 'app-http-page',
  imports: [
    CloseRequestDialogComponent,
    HttpRailComponent,
    RequestEditorComponent,
    RequestTabsComponent,
    ResponsePaneComponent,
    SaveRequestDialogComponent,
    TranslocoPipe,
  ],
  templateUrl: './http-page.component.html',
  styleUrl: './http-page.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
  host: { '(document:keydown)': 'onKeydown($event)' },
})
export class HttpPageComponent implements OnInit {
  protected readonly collections = inject(HttpCollectionsStore);
  protected readonly tabs = inject(HttpTabsStore);
  protected readonly settings = inject(SettingsStore);
  private readonly sending = inject(HttpSendStore);

  /** The tab a never-saved request is being placed for, and whether it closes once saved. */
  protected readonly placing = signal<{ readonly tab: RequestTab; readonly thenClose: boolean } | null>(null);
  protected readonly closing = signal<RequestTab | null>(null);

  ngOnInit(): void {
    void this.open();
  }

  /** A tab whose request went while the page was away closes; one never read is left alone. */
  private async open(): Promise<void> {
    const [read] = await Promise.all([this.collections.load(), this.tabs.restore()]);
    if (read) this.tabs.prune(this.collections.tree());
  }

  protected askSave(tab: RequestTab, thenClose = false): void {
    this.placing.set({ tab, thenClose });
  }

  protected async place(answer: SaveAnswer): Promise<void> {
    const placing = this.placing();
    if (placing === null) return;
    this.placing.set(null);
    this.tabs.edit(placing.tab.key, { name: answer.name });
    const key = await this.tabs.save(placing.tab.key, answer);
    if (key === null) return;
    await this.collections.load();
    if (placing.thenClose) this.tabs.close(key);
  }

  protected async onCloseChoice(choice: CloseChoice): Promise<void> {
    const tab = this.closing();
    if (tab === null) return;
    this.closing.set(null);
    if (choice === 'discard') {
      this.tabs.close(tab.key);
    } else if (tab.requestId === null) {
      this.askSave(tab, true);
    } else if ((await this.tabs.save(tab.key)) !== null) {
      this.tabs.close(tab.key);
    }
  }

  protected onKeydown(event: KeyboardEvent): void {
    if (event.defaultPrevented || event.altKey || event.shiftKey || !(event.ctrlKey || event.metaKey)) return;
    const key = event.key.toLowerCase();
    if (key === 'b') {
      event.preventDefault();
      this.settings.showLibraryRail.write(!this.settings.showLibraryRail());
    } else if (key === 'enter') {
      event.preventDefault();
      const tab = this.tabs.active();
      if (tab !== null) void this.sending.send(tab);
    } else if (key === 's') {
      event.preventDefault();
      const tab = this.tabs.active();
      if (tab === null || this.placing() !== null) return;
      if (tab.requestId === null) this.askSave(tab);
      else if (this.tabs.isDirty(tab)) void this.tabs.save(tab.key);
    }
  }
}
