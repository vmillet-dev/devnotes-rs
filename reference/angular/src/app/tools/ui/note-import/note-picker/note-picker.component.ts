import {
  ChangeDetectionStrategy,
  Component,
  ElementRef,
  afterNextRender,
  computed,
  inject,
  input,
  output,
  signal,
  viewChild,
} from '@angular/core';
import { TranslocoPipe, TranslocoService } from '@jsverse/transloco';
import { toSignal } from '@angular/core/rxjs-interop';
import { NotesRepository } from '@core/data/notes.repository';
import { noteCopyText } from '@core/model/checklist.model';
import { LANGUAGE_FULL_NAMES, LanguageTag } from '@core/model/language.model';
import { Note } from '@core/model/note.model';
import { ErrorNotifier } from '@core/services/errors/error-notifier.service';
import { NoteInput } from '@core/services/tools/tool.model';
import { liveResult } from '@core/services/tools/live-result';
import { ClockService } from '@core/services/time/clock.service';
import { SpacesStore } from '@core/state/spaces.store';
import { KindBadgeComponent } from '@shared/badges/kind-badge/kind-badge.component';
import { DialogComponent } from '@shared/layout/dialog/dialog.component';

export interface PickedNote {
  readonly title: string;
  readonly content: string;
  /** A snippet's alone: the other kinds have none. */
  readonly language: LanguageTag | null;
}

/** Enough to find one by its title: a search narrows it further. */
const SHOWN = 30;

/** The notes of every space an input accepts, searched like the canvas searches; the one chosen read whole. */
@Component({
  selector: 'app-note-picker',
  imports: [DialogComponent, KindBadgeComponent, TranslocoPipe],
  templateUrl: './note-picker.component.html',
  styleUrl: './note-picker.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class NotePickerComponent {
  private readonly notes = inject(NotesRepository);
  private readonly clock = inject(ClockService);
  private readonly spaces = inject(SpacesStore);
  private readonly notifier = inject(ErrorNotifier);
  private readonly transloco = inject(TranslocoService);

  /** Not required: the first request is built before an input is set, and asks nothing until then. */
  readonly accepts = input<NoteInput>();

  readonly picked = output<PickedNote>();
  readonly closed = output<void>();

  private readonly searchField = viewChild.required<ElementRef<HTMLInputElement>>('searchField');
  private readonly list = viewChild.required<ElementRef<HTMLElement>>('list');

  private readonly lang = toSignal(this.transloco.langChanges$, {
    initialValue: this.transloco.getActiveLang(),
  });

  protected readonly search = signal('');

  private readonly view = liveResult(
    () => {
      const accepts = this.accepts();
      return accepts && { search: this.search().trim(), kinds: accepts.kinds, languages: accepts.languages };
    },
    ({ search, kinds, languages }) =>
      this.notes.query({
        spaceId: null,
        folderId: null,
        search,
        filter: 'all',
        tags: [],
        languages: [...languages],
        kinds: [...kinds],
        priorities: [],
        now: this.clock.now(),
        tzOffsetMinutes: this.clock.now().getTimezoneOffset(),
        pinnedFirst: false,
      }),
  );

  protected readonly pending = this.view.pending;

  protected readonly found = computed(() =>
    (this.view.value()?.sections ?? []).flatMap((section) => section.notes).slice(0, SHOWN),
  );

  /** « Snippets JSON, YAML, TOML ou XML »: what the list holds, before anything is typed. */
  protected readonly accepted = computed(() => {
    const languages = this.accepts()?.languages ?? [];
    if (languages.length === 0) return { key: 'tools.fromNote.anyNote', params: {} };

    const names = languages.map((language) => LANGUAGE_FULL_NAMES[language]);
    const list = new Intl.ListFormat(this.lang(), { type: 'disjunction' }).format(names);
    return { key: 'tools.fromNote.snippets', params: { languages: list } };
  });

  private readonly spaceNames = computed(
    () => new Map(this.spaces.spaces().map((space) => [space.id, space.name] as const)),
  );

  constructor() {
    afterNextRender(() => this.searchField().nativeElement.focus());
  }

  protected whereOf(note: Note): string {
    const space = this.spaceNames().get(note.spaceId) ?? '';
    return note.folder ? `${space} › ${note.folder.name}` : space;
  }

  protected onSearch(event: Event): void {
    this.search.set((event.target as HTMLInputElement).value);
  }

  /** Enter takes the first, as the palette's does; the down arrow goes into the list. */
  protected onSearchKeydown(event: KeyboardEvent): void {
    if (event.key === 'Enter') {
      const first = this.found()[0];
      if (first !== undefined) {
        event.preventDefault();
        void this.pick(first);
      }
    } else if (event.key === 'ArrowDown') {
      event.preventDefault();
      this.rows()[0]?.focus();
    }
  }

  /** Up from the first row goes back to the field, where typing goes on. */
  protected onRowKeydown(event: KeyboardEvent, index: number): void {
    const step = event.key === 'ArrowDown' ? 1 : event.key === 'ArrowUp' ? -1 : 0;
    if (step === 0) return;

    event.preventDefault();
    const next = index + step;
    if (next < 0) {
      this.searchField().nativeElement.focus();
    } else {
      this.rows()[Math.min(next, this.rows().length - 1)]?.focus();
    }
  }

  /** A list sends previews: the body is read whole, and a todo list gives its Markdown. */
  protected async pick(note: Note): Promise<void> {
    const whole = await this.notifier.attempt('errors.noteReadFailed', () => this.notes.whole(note));
    if (whole === null) return;

    const language = whole.kind === 'snippet' ? whole.language : null;
    this.picked.emit({ title: whole.title, content: noteCopyText(whole), language });
  }

  private rows(): HTMLButtonElement[] {
    return [...this.list().nativeElement.querySelectorAll<HTMLButtonElement>('button.note')];
  }
}
