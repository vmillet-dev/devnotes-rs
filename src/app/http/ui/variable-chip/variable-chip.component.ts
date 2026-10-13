import { ChangeDetectionStrategy, Component, computed, input, output, signal } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { UsedVariable } from '@core/model/http-environment.model';
import { IconComponent } from '@shared/icon/icon.component';

/**
 * A `{{name}}` the request holds, in the colour of where it resolves from — red when nothing
 * sets it. Pointed at or focused, it says from where and its value, a secret masked until asked.
 */
@Component({
  selector: 'app-variable-chip',
  imports: [IconComponent, TranslocoPipe],
  templateUrl: './variable-chip.component.html',
  styleUrl: './variable-chip.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class VariableChipComponent {
  readonly variable = input.required<UsedVariable>();
  /** What « Définir dans … » names: the active environment, or the globals with none. */
  readonly defineIn = input<string | null>(null);

  readonly defined = output<string>();

  protected readonly revealed = signal(false);
  protected readonly secret = computed(() => this.variable().resolved?.kind === 'secret');
  protected readonly tipId = computed(() => `http-variable-tip-${this.variable().name}`);
  /** As typed in the request: braces and all. */
  protected readonly written = computed(() => `{{${this.variable().name}}}`);

  protected toggleReveal(): void {
    this.revealed.update((shown) => !shown);
  }

  /** Leaving the chip masks the secret again: shown for that look, not for the session. */
  protected leave(event: FocusEvent | PointerEvent): void {
    const host = event.currentTarget as HTMLElement;
    if (event instanceof FocusEvent && host.contains(event.relatedTarget as Node | null)) return;
    this.revealed.set(false);
  }
}
