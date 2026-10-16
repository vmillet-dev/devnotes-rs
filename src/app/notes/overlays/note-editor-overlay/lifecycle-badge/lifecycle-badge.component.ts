import { ChangeDetectionStrategy, Component, computed, inject, input } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { NoteLifecycle } from '@core/model/note.model';
import { TranslationRef } from '@core/services/i18n/translation-ref.model';
import { ClockService } from '@core/services/time/clock.service';
import { expiryRef } from '@core/utils/relative-time.util';

@Component({
  selector: 'app-lifecycle-badge',
  imports: [TranslocoPipe],
  templateUrl: './lifecycle-badge.component.html',
  styleUrl: './lifecycle-badge.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class LifecycleBadgeComponent {
  private readonly clock = inject(ClockService);

  readonly lifecycle = input.required<NoteLifecycle>();

  /** As the back end decided it: a threshold recomputed here would be free to drift. */
  readonly expiringSoon = input(false);

  /** Not the pin's 📌, which the « Épingler » button beside it wears. */
  protected readonly icon = computed(() => (this.lifecycle().kind === 'permanent' ? '∞' : '⏳'));

  protected readonly label = computed<TranslationRef>(() => {
    const lifecycle = this.lifecycle();
    return lifecycle.kind === 'permanent'
      ? { key: 'lifecycle.neverExpires' }
      : expiryRef(lifecycle.at, this.clock.now());
  });
}
