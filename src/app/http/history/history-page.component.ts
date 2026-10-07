import { ChangeDetectionStrategy, Component, computed, inject, signal } from '@angular/core';
import { TranslocoPipe, TranslocoService } from '@jsverse/transloco';
import { HistoryItem } from '@core/model/http.model';
import { errorKeyOf } from '@core/services/errors/error-notifier.service';
import { HttpHistoryStore } from '@core/services/http/http-history.store';
import { ClockService } from '@core/services/time/clock.service';
import { endOfLocalDay, toDateInputValue } from '@core/utils/local-day.util';
import { ResponseViewComponent } from '@http/ui/response-view/response-view.component';

/** What was sent, by day, beside the entry chosen read like a response. */
@Component({
  selector: 'app-history-page',
  imports: [ResponseViewComponent, TranslocoPipe],
  templateUrl: './history-page.component.html',
  styleUrl: './history-page.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class HistoryPageComponent {
  protected readonly history = inject(HttpHistoryStore);
  private readonly clock = inject(ClockService);
  private readonly transloco = inject(TranslocoService);

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

  protected failureKey(item: HistoryItem): string {
    const code = item.summary.failure;
    return (code !== null && errorKeyOf(code)) || 'http.send.failed';
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
