import { ChangeDetectionStrategy, Component, computed, inject, input } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { HttpSendStore } from '@core/services/http/http-send.store';
import { ResponseViewComponent } from '@http/ui/response-view/response-view.component';

/** The lower half of a request's tab: where its last send stands, and what it answered. */
@Component({
  selector: 'app-response-pane',
  imports: [ResponseViewComponent, TranslocoPipe],
  templateUrl: './response-pane.component.html',
  styleUrl: './response-pane.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ResponsePaneComponent {
  readonly tabKey = input.required<string>();
  readonly requestName = input.required<string>();

  private readonly sending = inject(HttpSendStore);

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

  protected saveFile(): void {
    void this.sending.saveBody(this.tabKey());
  }
}
