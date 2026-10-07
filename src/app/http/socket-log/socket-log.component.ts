import { ChangeDetectionStrategy, Component, computed, inject, input, signal } from '@angular/core';
import { TranslocoPipe, TranslocoService } from '@jsverse/transloco';
import { HttpSocketsStore, SocketEntry } from '@core/services/http/http-sockets.store';

/** What a row reads as: its direction, and the words the filter looks in. */
function textOf(entry: SocketEntry): string {
  switch (entry.event.kind) {
    case 'sent':
    case 'received':
      return entry.event.text;
    case 'opened':
      return entry.event.url;
    case 'closed':
      return entry.event.reason;
    case 'failed':
      return entry.event.detail;
  }
}

/** The lower half of a WebSocket's tab: where the socket stands, and what went each way. */
@Component({
  selector: 'app-socket-log',
  imports: [TranslocoPipe],
  templateUrl: './socket-log.component.html',
  styleUrl: './socket-log.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class SocketLogComponent {
  readonly tabKey = input.required<string>();

  protected readonly sockets = inject(HttpSocketsStore);
  private readonly transloco = inject(TranslocoService);

  protected readonly filter = signal('');
  protected readonly state = computed(() => this.sockets.states().get(this.tabKey()) ?? null);
  protected readonly phase = computed(() => this.state()?.phase ?? 'closed');
  protected readonly entries = computed(() => {
    const words = this.filter().trim().toLowerCase();
    const log = this.state()?.log ?? [];
    return words === '' ? log : log.filter((entry) => textOf(entry).toLowerCase().includes(words));
  });

  protected text(entry: SocketEntry): string {
    return textOf(entry);
  }

  protected time(entry: SocketEntry): string {
    return new Intl.DateTimeFormat(this.transloco.activeLang(), {
      hour: '2-digit',
      minute: '2-digit',
      second: '2-digit',
      fractionalSecondDigits: 3,
    }).format(entry.at);
  }

  protected onFilter(event: Event): void {
    this.filter.set((event.target as HTMLInputElement).value);
  }
}
