import {
  ChangeDetectionStrategy,
  Component,
  ElementRef,
  Injector,
  afterNextRender,
  computed,
  inject,
  input,
  linkedSignal,
  output,
  signal,
  viewChild,
} from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { HttpCollectionsStore } from '@core/services/http/http-collections.store';
import { savePlaces } from '@core/services/http/http-tree';
import { SaveTarget } from '@core/services/http/http-tabs.store';
import { ChoiceMenuComponent, ChoiceOption } from '@shared/controls/choice-menu/choice-menu.component';
import { DialogComponent } from '@shared/layout/dialog/dialog.component';
import { DialogLayer } from '@shared/layout/dialog/dialog.model';

export interface SaveAnswer extends SaveTarget {
  readonly name: string;
}

/** What is missing from the form, said under the field that lacks it. */
type Missing = 'name' | 'collection' | 'folder';

/** The menu's last entry. No place has it for an id: theirs are Rust's. */
const NEW_COLLECTION = 'new-collection';

/**
 * Where a request made from « Nouvelle requête » goes the first time it is saved, and under
 * which name. It creates what it lacks: on a library with no collection it used to send its
 * reader to the rail and back, so the first request of a project needed three trips.
 */
@Component({
  selector: 'app-save-request-dialog',
  imports: [ChoiceMenuComponent, DialogComponent, TranslocoPipe],
  templateUrl: './save-request-dialog.component.html',
  styleUrl: './save-request-dialog.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class SaveRequestDialogComponent {
  readonly name = input.required<string>();
  /** Raised when it is asked from a question that already stands above the page's dialogs. */
  readonly layer = input<DialogLayer>('app');
  readonly saved = output<SaveAnswer>();
  readonly cancelled = output<void>();

  private readonly collections = inject(HttpCollectionsStore);
  private readonly injector = inject(Injector);
  private readonly nameInput = viewChild<ElementRef<HTMLInputElement>>('nameInput');
  private readonly collectionInput = viewChild<ElementRef<HTMLInputElement>>('collectionInput');
  private readonly folderInput = viewChild<ElementRef<HTMLInputElement>>('folderInput');

  protected readonly places = computed(() => savePlaces(this.collections.tree()));
  protected readonly options = computed<readonly ChoiceOption[]>(() => [
    ...this.places().map((place) => ({ id: place.id, name: place.path.join(' / ') })),
    { id: NEW_COLLECTION, name: 'http.save.newCollection', nameIsKey: true },
  ]);
  /** With no collection there is nothing to choose: the new one is the only place. */
  protected readonly placeId = linkedSignal<string>(() => this.places()[0]?.id ?? NEW_COLLECTION);
  protected readonly newCollection = computed(() => this.placeId() === NEW_COLLECTION);
  /** A folder to create under the place chosen; another place, and it is asked again. */
  protected readonly newFolder = linkedSignal({ source: this.placeId, computation: () => false });
  protected readonly missing = signal<Missing | null>(null);

  constructor() {
    afterNextRender(() => this.nameInput()?.nativeElement.select());
  }

  protected choosePlace(id: string | null): void {
    if (id === null) return;
    this.placeId.set(id);
    this.missing.set(null);
    if (id === NEW_COLLECTION) this.focus(this.collectionInput);
  }

  protected askFolder(asked: boolean): void {
    this.newFolder.set(asked);
    this.missing.set(null);
    if (asked) this.focus(this.folderInput);
  }

  protected submit(event: Event, typed: string): void {
    event.preventDefault();
    const name = typed.trim();
    if (name === '') return this.refuse('name', this.nameInput);

    if (this.newCollection()) {
      const collection = this.collectionInput()?.nativeElement.value.trim() ?? '';
      if (collection === '') return this.refuse('collection', this.collectionInput);
      this.saved.emit({ collectionId: '', folderId: null, name, create: { collection } });
      return;
    }

    const place = this.places().find((candidate) => candidate.id === this.placeId());
    if (!place) return;
    const target = { collectionId: place.collectionId, folderId: place.folderId, name };
    if (!this.newFolder()) {
      this.saved.emit(target);
      return;
    }
    const folder = this.folderInput()?.nativeElement.value.trim() ?? '';
    if (folder === '') return this.refuse('folder', this.folderInput);
    this.saved.emit({ ...target, create: { folder } });
  }

  /** Refused in place: the field that lacks its name takes the focus, and says so. */
  private refuse(missing: Missing, field: () => ElementRef<HTMLInputElement> | undefined): void {
    this.missing.set(missing);
    field()?.nativeElement.focus();
  }

  /** The field is drawn by the change that asks for it: focused once it is there. */
  private focus(field: () => ElementRef<HTMLInputElement> | undefined): void {
    afterNextRender(() => field()?.nativeElement.focus(), { injector: this.injector });
  }
}
