import { NgTemplateOutlet } from '@angular/common';
import { ChangeDetectionStrategy, Component, computed, effect, inject, input, signal } from '@angular/core';
import { TranslocoPipe, TranslocoService } from '@jsverse/transloco';
import { HttpSendStore } from '@core/services/http/http-send.store';
import { ResponseViewComponent } from '@http/ui/response-view/response-view.component';
import { SpinnerComponent } from '@shared/spinner/spinner.component';

/** Often enough for the tenths to move, which is what tells a slow server from a frozen window. */
const ELAPSED_TICK_MS = 100;

/** The lower half of a request's tab: where its last send stands, and what it answered. */
@Component({
  selector: 'app-response-pane',
  imports: [NgTemplateOutlet, ResponseViewComponent, SpinnerComponent, TranslocoPipe],
  templateUrl: './response-pane.component.html',
  styleUrl: './response-pane.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ResponsePaneComponent {
  readonly tabKey = input.required<string>();
  readonly requestName = input.required<string>();

  private readonly sending = inject(HttpSendStore);
  private readonly transloco = inject(TranslocoService);

  protected readonly state = computed(() => this.sending.states().get(this.tabKey()) ?? null);
  protected readonly phase = computed(() => this.state()?.phase ?? 'idle');
  protected readonly sendId = computed(() => this.state()?.sendId ?? null);
  protected readonly response = computed(() => {
    const state = this.state();
    return state?.phase === 'answered' ? state.response : null;
  });
  protected readonly failure = computed(() => {
    const state = this.state();
    return state?.phase === 'failed' ? state.notice.ref : null;
  });

  /** Moves only while a send is under way: nothing ticks under an answer. */
  private readonly now = signal(Date.now());

  /** « 2,4 s », counted from the send and not from when this pane was drawn. */
  protected readonly elapsed = computed(() => {
    const state = this.state();
    if (state?.phase !== 'sending') return null;

    const seconds = Math.max(0, this.now() - state.startedAt) / 1000;
    const tenths = new Intl.NumberFormat(this.transloco.getActiveLang(), {
      minimumFractionDigits: 1,
      maximumFractionDigits: 1,
    }).format(seconds);
    return `${tenths} s`;
  });

  constructor() {
    effect((onCleanup) => {
      if (this.phase() !== 'sending') return;

      this.now.set(Date.now());
      const ticking = setInterval(() => this.now.set(Date.now()), ELAPSED_TICK_MS);
      onCleanup(() => clearInterval(ticking));
    });
  }

  protected saveFile(): void {
    void this.sending.saveBody(this.tabKey());
  }

  /** The pane, not the record: the history keeps the send. */
  protected clear(): void {
    this.sending.forget(this.tabKey());
  }
}
