import { ChangeDetectionStrategy, Component, computed, inject, signal } from '@angular/core';
import { TranslocoPipe, TranslocoService } from '@jsverse/transloco';
import { HistoryEntry, HistoryItem } from '@core/model/http.model';
import { errorKeyOf } from '@core/services/errors/error-notifier.service';
import { TranslationRef } from '@core/services/i18n/translation-ref.model';
import { HttpHistoryStore } from '@core/services/http/http-history.store';
import { HISTORY_LIST_WIDTH } from '@core/services/settings/app-settings.model';
import { SettingsStore } from '@core/services/settings/settings.store';
import { ClockService } from '@core/services/time/clock.service';
import { endOfLocalDay, toDateInputValue } from '@core/utils/local-day.util';
import { ResponseViewComponent } from '@http/ui/response-view/response-view.component';
import { RailToggleComponent } from '@shared/controls/rail-toggle/rail-toggle.component';
import { RailFrameComponent } from '@shared/layout/rail-frame/rail-frame.component';

/** What was sent, by day, beside the entry chosen read like a response. */
@Component({
  selector: 'app-history-page',
  imports: [RailFrameComponent, RailToggleComponent, ResponseViewComponent, TranslocoPipe],
  templateUrl: './history-page.component.html',
  styleUrl: './history-page.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class HistoryPageComponent {
  protected readonly history = inject(HttpHistoryStore);
  protected readonly settings = inject(SettingsStore);
  private readonly clock = inject(ClockService);
  private readonly transloco = inject(TranslocoService);

  protected readonly listWidth = HISTORY_LIST_WIDTH;

  /** The count Rust gave, while « Vider » asks; `null` otherwise. */
  protected readonly confirming = signal<number | null>(null);
  protected readonly total = computed(() =>
    this.history.days().reduce((count, day) => count + day.items.length, 0),
  );

  protected dayLabel(day: string): string {
    const now = this.clock.now();
    if (day === toDateInputValue(now)) return this.transloco.translate('http.history.today');
    const yesterday = new Date(now.getFullYear(), now.getMonth(), now.getDate() - 1);
    if (day === toDateInputValue(yesterday)) return this.transloco.translate('http.history.yesterday');
    const date = endOfLocalDay(day);
    return date === null
      ? day
      : new Intl.DateTimeFormat(this.transloco.activeLang(), {
          weekday: 'long',
          day: 'numeric',
          month: 'long',
        }).format(date);
  }

  protected time(item: HistoryItem): string {
    return new Intl.DateTimeFormat(this.transloco.activeLang(), {
      hour: '2-digit',
      minute: '2-digit',
      second: '2-digit',
    }).format(new Date(item.sentAt));
  }

  protected statusClass(item: HistoryItem): string {
    return item.status === null ? 'failed' : String(Math.floor(item.status / 100));
  }

  /** The sentence the send itself showed: the code, and the values it stated. */
  protected failure(item: HistoryItem): TranslationRef {
    const { failure: code, failureParams: params = {} } = item.summary;
    return { key: (code !== null && errorKeyOf(code, params)) || 'http.send.failed', params };
  }

  /** Which « Rouvrir » it will be: back in its request while it exists, a draft otherwise. */
  protected reopening(entry: HistoryEntry): TranslationRef {
    const held = this.history.requestOf(entry.item);
    return held === null
      ? { key: 'http.history.reopenDraft' }
      : { key: 'http.history.reopenInto', params: { name: held.name } };
  }

  /** `Suppr` on the focused row deletes it, as its button does. */
  protected onItemKey(event: KeyboardEvent, item: HistoryItem): void {
    if (event.key !== 'Delete' || event.ctrlKey || event.metaKey || event.altKey) return;
    event.preventDefault();
    void this.history.delete(item.id);
  }

  protected async askClear(): Promise<void> {
    const count = await this.history.count();
    if (count !== null) this.confirming.set(count);
  }

  protected async clear(): Promise<void> {
    this.confirming.set(null);
    await this.history.clear();
  }
}
