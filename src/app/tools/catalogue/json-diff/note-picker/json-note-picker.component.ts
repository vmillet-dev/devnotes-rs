import {
  ChangeDetectionStrategy,
  Component,
  ElementRef,
  afterNextRender,
  computed,
  inject,
  output,
  signal,
  viewChild,
} from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { NotesRepository } from '@core/data/notes.repository';
import { Note } from '@core/model/note.model';
import { liveResult } from '@core/services/tools/live-result';
import { ClockService } from '@core/services/time/clock.service';
import { DialogComponent } from '@shared/layout/dialog/dialog.component';

export interface PickedNote {
  readonly title: string;
  readonly content: string;
}

/** Enough to find one by its title: a search narrows it further. */
const SHOWN = 30;

/** The JSON snippets of every space, searched like the canvas searches; the one chosen read whole. */
@Component({
  selector: 'app-json-note-picker',
  imports: [DialogComponent, TranslocoPipe],
  templateUrl: './json-note-picker.component.html',
  styleUrl: './json-note-picker.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class JsonNotePickerComponent {
  private readonly notes = inject(NotesRepository);
  private readonly clock = inject(ClockService);

  readonly picked = output<PickedNote>();
  readonly closed = output<void>();

  private readonly searchField = viewChild.required<ElementRef<HTMLInputElement>>('searchField');

  protected readonly search = signal('');

  private readonly view = liveResult(
    () => ({ search: this.search().trim() }),
    ({ search }) =>
      this.notes.query({
        spaceId: null,
        folderId: null,
        search,
        filter: 'all',
        tags: [],
        languages: ['json'],
        kinds: ['snippet'],
        priorities: [],
        now: this.clock.now(),
        tzOffsetMinutes: new Date().getTimezoneOffset(),
        pinnedFirst: false,
      }),
  );

  protected readonly found = computed(() =>
    (this.view.value()?.sections ?? []).flatMap((section) => section.notes).slice(0, SHOWN),
  );

  constructor() {
    afterNextRender(() => this.searchField().nativeElement.focus());
  }

  protected onSearch(event: Event): void {
    this.search.set((event.target as HTMLInputElement).value);
  }

  /** A list sends previews: the body is read whole before it is compared. */
  protected async pick(note: Note): Promise<void> {
    const whole = await this.notes.whole(note);
    this.picked.emit({ title: whole.title, content: whole.content });
  }
}
