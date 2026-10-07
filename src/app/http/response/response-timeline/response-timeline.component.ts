import { ChangeDetectionStrategy, Component, computed, input } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { SentResponse } from '@core/model/http.model';

/**
 * The redirects followed, then the exchange as raw text: what left and what came back. No DNS,
 * connect and TLS breakdown — reqwest measures none.
 */
@Component({
  selector: 'app-response-timeline',
  imports: [TranslocoPipe],
  templateUrl: './response-timeline.component.html',
  styleUrl: './response-timeline.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ResponseTimelineComponent {
  readonly response = input.required<SentResponse>();

  protected readonly sent = computed(() => {
    const { method, url, headers, body } = this.response().exchange;
    const lines = [`${method} ${url}`, ...headers.map((header) => `${header.key}: ${header.value}`)];
    return { head: lines.join('\n'), body };
  });

  protected readonly received = computed(() => {
    const response = this.response();
    return [
      `${response.status} ${response.reason}`.trim(),
      ...response.headers.map((header) => `${header.key}: ${header.value}`),
    ].join('\n');
  });
}
