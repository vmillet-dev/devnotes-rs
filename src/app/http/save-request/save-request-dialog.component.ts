import {
  ChangeDetectionStrategy,
  Component,
  ElementRef,
  afterNextRender,
  computed,
  inject,
  input,
  linkedSignal,
  output,
  viewChild,
} from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { HttpCollectionsStore } from '@core/services/http/http-collections.store';
import { savePlaces } from '@core/services/http/http-tree';
import { SaveTarget } from '@core/services/http/http-tabs.store';
import { ChoiceMenuComponent, ChoiceOption } from '@shared/controls/choice-menu/choice-menu.component';
import { DialogComponent } from '@shared/layout/dialog/dialog.component';

export interface SaveAnswer extends SaveTarget {
  readonly name: string;
}

/** Where a request made from the `+` goes the first time it is saved, and under which name. */
@Component({
  selector: 'app-save-request-dialog',
  imports: [ChoiceMenuComponent, DialogComponent, TranslocoPipe],
  templateUrl: './save-request-dialog.component.html',
  styleUrl: './save-request-dialog.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class SaveRequestDialogComponent {
  readonly name = input.required<string>();
  readonly saved = output<SaveAnswer>();
  readonly cancelled = output<void>();

  private readonly collections = inject(HttpCollectionsStore);
  private readonly nameInput = viewChild<ElementRef<HTMLInputElement>>('nameInput');

  protected readonly places = computed(() => savePlaces(this.collections.tree()));
  protected readonly options = computed<readonly ChoiceOption[]>(() =>
    this.places().map((place) => ({ id: place.id, name: place.path.join(' / ') })),
  );
  protected readonly placeId = linkedSignal<string | null>(() => this.places()[0]?.id ?? null);

  constructor() {
    afterNextRender(() => this.nameInput()?.nativeElement.select());
  }

  protected submit(event: Event, name: string): void {
    event.preventDefault();
    const place = this.places().find((candidate) => candidate.id === this.placeId());
    if (!place || name.trim() === '') return;
    this.saved.emit({ collectionId: place.collectionId, folderId: place.folderId, name: name.trim() });
  }
}
