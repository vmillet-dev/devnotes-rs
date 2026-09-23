import { ChangeDetectionStrategy, Component, PendingTasks, inject } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { VariablesStore } from '@core/state/variables.store';
import { Variable, isVariableName } from '@core/model/variable.model';

function typedValue(event: Event): string {
  return (event.target as HTMLInputElement).value;
}

/** These values are only a suggestion: what was typed on a note wins. */
@Component({
  selector: 'app-variables-page',
  imports: [TranslocoPipe],
  templateUrl: './variables-page.component.html',
  styleUrl: './variables-page.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class VariablesPageComponent {
  protected readonly store = inject(VariablesStore);

  constructor() {
    // A pending task, so `whenStable` waits for the read rather than for a guess at it.
    void inject(PendingTasks).run(() => this.store.load());
  }

  /** Empty until something is typed: a new row is not a mistake. */
  protected isRejected(variable: Variable): boolean {
    return variable.name !== '' && !isVariableName(variable.name);
  }

  protected onName(index: number, event: Event): void {
    this.store.rename(index, typedValue(event));
  }

  protected onValue(index: number, event: Event): void {
    this.store.setValue(index, typedValue(event));
  }

  protected remove(index: number): void {
    this.store.remove(index);
  }
}
