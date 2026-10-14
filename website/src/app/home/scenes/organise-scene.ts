import { ChangeDetectionStrategy, Component, computed, inject } from '@angular/core';

import { field, part, typed } from '../../core/browser';
import { ScenePlayer } from '../scene-player';

const SEARCH = 'pg_stat';

/** A space whose folders are zones on the board; a zone grows, a search, then the list by date. */
@Component({
  selector: 'app-organise-scene',
  templateUrl: './organise-scene.html',
  styleUrl: './organise-scene.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
  host: {
    '[class.resizing]': 'resizing()',
    '[class.typing]': 'typing()',
    '[class.searching]': 'search() === searched',
    '[class.listed]': 'listed()',
  },
})
export class OrganiseScene {
  private readonly player = inject(ScenePlayer);
  private readonly step = this.player.step;
  private readonly t = this.player.t;
  protected readonly field = field;
  protected readonly searched = SEARCH;

  /** How much wider the database folder is than its neighbours. */
  protected readonly growth = computed(() => {
    const step = this.step();
    if (step === 0) return 1;
    return step === 1 ? 1 + 0.75 * part(this.t(), 0.1, 0.8) : 1.75;
  });
  protected readonly resizing = computed(() => this.step() === 1 && this.t() > 0.05 && this.t() < 0.9);
  protected readonly search = computed(() =>
    this.step() === 2 ? typed(SEARCH, part(this.t(), 0.05, 0.5)) : '',
  );
  protected readonly typing = computed(() => this.step() === 2 && this.t() < 0.55);
  protected readonly listed = computed(() => this.step() === 3 && this.t() > 0.12);
}
