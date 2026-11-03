import {
  ChangeDetectionStrategy,
  Component,
  computed,
  inject,
  input,
  linkedSignal,
  output,
} from '@angular/core';
import { TranslocoPipe, TranslocoService } from '@jsverse/transloco';
import {
  RUNNER_FILTERS,
  RunData,
  RunResult,
  RunnerFilter,
  byIteration,
  failed,
  summaryOf,
} from '@core/model/http-runner.model';
import { requestBadge, shortBadge } from '@core/model/http.model';
import { errorKeyOf } from '@core/services/errors/error-notifier.service';
import { ResponseViewComponent } from '@http/ui/response-view/response-view.component';

/** A run's report: what failed, how many tests passed, each iteration's requests. */
@Component({
  selector: 'app-runner-results',
  imports: [ResponseViewComponent, TranslocoPipe],
  templateUrl: './runner-results.component.html',
  styleUrl: './runner-results.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class RunnerResultsComponent {
  readonly results = input.required<readonly RunResult[]>();
  readonly iterations = input.required<number>();
  readonly data = input<RunData | null>(null);
  readonly millis = input<number | null>(null);
  /** Cut short by « Annuler »: how many requests it was to send, to say how far it went. */
  readonly cancelledOf = input<number | null>(null);
  readonly filter = input.required<RunnerFilter>();
  readonly filterChanged = output<RunnerFilter>();

  private readonly transloco = inject(TranslocoService);

  protected readonly filters = RUNNER_FILTERS;
  protected readonly summary = computed(() => summaryOf(this.results()));
  protected readonly counts = computed<Record<RunnerFilter, number>>(() => {
    const results = this.results();
    const failures = results.filter(failed).length;
    return { all: results.length, passed: results.length - failures, failed: failures };
  });
  protected readonly groups = computed(() => {
    const filter = this.filter();
    return byIteration(this.results()).map((group) => ({
      ...group,
      results: group.results.filter((result) => filter === 'all' || (filter === 'failed') === failed(result)),
    }));
  });
  /** A failed request opens on its assertions; a click opens or folds any other. */
  protected readonly open = linkedSignal<readonly RunResult[], ReadonlySet<RunResult>>({
    source: this.results,
    computation: (results, previous) => {
      const open = new Set(previous?.value ?? []);
      for (const result of results) if (failed(result)) open.add(result);
      return open;
    },
  });

  protected isFailed(result: RunResult): boolean {
    return failed(result);
  }

  protected badge(result: RunResult): string {
    return shortBadge(requestBadge(result));
  }

  protected toggle(result: RunResult): void {
    const open = new Set(this.open());
    if (!open.delete(result)) open.add(result);
    this.open.set(open);
  }

  protected rowOf(iteration: number): string {
    const row = this.data()?.rows[iteration];
    return row
      ? Object.entries(row)
          .map(([key, value]) => `${key} = ${value}`)
          .join(' · ')
      : '';
  }

  protected passedOf(result: RunResult): string {
    const ran = result.tests.filter((test) => !test.skipped);
    return ran.length === 0 ? '' : `${ran.filter((test) => test.passed).length}/${ran.length}`;
  }

  protected errorOf(result: RunResult): { key: string; params: Record<string, string> } | null {
    if (result.error === null) return null;
    return {
      key: errorKeyOf(result.error.code, result.error.params) ?? 'http.send.failed',
      params: result.error.params,
    };
  }

  protected seconds(millis: number): string {
    return new Intl.NumberFormat(this.transloco.getActiveLang(), {
      maximumFractionDigits: 1,
      minimumFractionDigits: 1,
    }).format(millis / 1000);
  }
}
