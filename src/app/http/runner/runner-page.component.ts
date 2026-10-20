import { ChangeDetectionStrategy, Component, computed, inject } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { requestBadge } from '@core/model/http.model';
import { HttpEnvironmentsStore } from '@core/services/http/http-environments.store';
import { HttpRunnerStore } from '@core/services/http/http-runner.store';
import { SettingsStore } from '@core/services/settings/settings.store';
import { RailToggleComponent } from '@shared/controls/rail-toggle/rail-toggle.component';
import { SpinnerComponent } from '@shared/spinner/spinner.component';
import { HttpConsoleComponent } from '@http/response/console/http-console.component';
import { ProtectedQuestionComponent } from '@http/ui/protected-question/protected-question.component';
import { RunnerOrderComponent } from './runner-order/runner-order.component';
import { RunnerResultsComponent } from './runner-results/runner-results.component';

/**
 * A collection or a folder run: how — iterations, a delay, a data file, three switches —, in
 * which order, and the report of the last run with its console. A page of the workspace, as the
 * history and the environments are.
 */
@Component({
  selector: 'app-runner-page',
  imports: [
    HttpConsoleComponent,
    ProtectedQuestionComponent,
    RailToggleComponent,
    RunnerOrderComponent,
    RunnerResultsComponent,
    SpinnerComponent,
    TranslocoPipe,
  ],
  templateUrl: './runner-page.component.html',
  styleUrl: './runner-page.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class RunnerPageComponent {
  protected readonly store = inject(HttpRunnerStore);
  protected readonly environments = inject(HttpEnvironmentsStore);
  protected readonly settings = inject(SettingsStore);

  protected readonly running = computed(() => this.store.phase() === 'running');
  /** « POST Login, POST Payer »: what the protected environment's question names. */
  protected readonly changingNames = computed(() =>
    this.store
      .changing()
      .map((row) => `${requestBadge(row)} ${row.name}`)
      .join(', '),
  );

  protected onNumber(event: Event, write: (value: number) => void): void {
    const value = Number.parseInt((event.target as HTMLInputElement).value, 10);
    if (!Number.isNaN(value)) write(Math.max(0, value));
  }

  protected onCheck(event: Event, write: (value: boolean) => void): void {
    write((event.target as HTMLInputElement).checked);
  }
}
