import { Injectable, computed, inject, signal } from '@angular/core';
import { CalcSheets } from '@core/services/calc/calc-sheets.service';
import { ClipboardService } from '@core/services/clipboard/clipboard.service';
import { DesktopNotifier } from '@core/services/notifications/desktop-notifier.service';
import { ErrorNotifier } from '@core/services/errors/error-notifier.service';
import { StatusNotifier } from '@core/services/notifications/status.service';
import { SettingsStore } from '@core/services/settings/settings.store';
import { SEARCH_DEBOUNCE_MS, debounced } from '@core/services/time/debounce';
import { ClockService } from '@core/services/time/clock.service';
import { AppWindowService } from '@core/services/window/app-window.service';
import { HttpRepository } from '@core/data/http.repository';
import { NotesRepository } from '@core/data/notes.repository';
import { noteCopyText } from '@core/model/checklist.model';
import { RequestHit } from '@core/model/http.model';
import { Note, TagUsage } from '@core/model/note.model';

/** What `@` narrows the search to, in the order the groups are drawn. */
export const PALETTE_KINDS = ['notes', 'requests', 'tools', 'spaces'] as const;
export type PaletteKind = (typeof PALETTE_KINDS)[number];

/** A chip in the field: a kind, one space's notes, or a tag's. */
export type PaletteChip =
  | { readonly kind: 'scope'; readonly scope: PaletteKind }
  | { readonly kind: 'space'; readonly id: string; readonly name: string }
  | { readonly kind: 'tag'; readonly tag: string };

/** What is typed after the chips asks for: a search, or a list to choose from. */
export type PaletteMode =
  | { readonly kind: 'search'; readonly term: string }
  | { readonly kind: 'scopes'; readonly word: string }
  | { readonly kind: 'tags'; readonly word: string }
  | { readonly kind: 'commands'; readonly term: string };

/** `@` and `#` are read where the word being typed starts, `>` only at the head of the field. */
export function paletteMode(text: string): PaletteMode {
  const at = /(?:^|\s)@(\S*)$/.exec(text);
  if (at) return { kind: 'scopes', word: at[1] ?? '' };

  const hash = /(?:^|\s)#(\S*)$/.exec(text);
  if (hash) return { kind: 'tags', word: hash[1] ?? '' };

  const command = /^\s*>(.*)$/.exec(text);
  if (command) return { kind: 'commands', term: (command[1] ?? '').trim() };

  return { kind: 'search', term: text.trim() };
}

/** Beyond this one kind no longer fits on screen beside the others. */
export const MAX_NOTES = 8;

/**
 * Queries every space and ignores the canvas filters. It does not reuse `NotesStore`
 * for the same reason: its search would change what the canvas shows behind it. What Rust
 * answers is here — the notes, the requests, the tags; the tools, the spaces and the commands
 * are read on the front by the palette, which knows their words.
 */
@Injectable({ providedIn: 'root' })
export class PaletteStore {
  private readonly notesRepository = inject(NotesRepository);
  private readonly httpRepository = inject(HttpRepository);
  private readonly clipboard = inject(ClipboardService);
  private readonly clock = inject(ClockService);
  private readonly window = inject(AppWindowService);
  private readonly notifier = inject(ErrorNotifier);
  private readonly desktop = inject(DesktopNotifier);
  private readonly status = inject(StatusNotifier);
  private readonly settings = inject(SettingsStore);
  private readonly calc = inject(CalcSheets);

  private readonly _isOpen = signal(false);
  private readonly _fromElsewhere = signal(false);
  private readonly _text = signal('');
  private readonly _chips = signal<readonly PaletteChip[]>([]);
  private readonly _notes = signal<readonly Note[]>([]);
  private readonly _requests = signal<readonly RequestHit[]>([]);
  private readonly _tags = signal<readonly TagUsage[]>([]);
  private readonly _pendingFill = signal<Note | null>(null);

  readonly isOpen = this._isOpen.asReadonly();
  /** Opened over another application: copying a note puts the window away again. */
  readonly fromElsewhere = this._fromElsewhere.asReadonly();
  readonly text = this._text.asReadonly();
  readonly chips = this._chips.asReadonly();
  readonly notes = this._notes.asReadonly();
  readonly requests = this._requests.asReadonly();
  readonly tags = this._tags.asReadonly();
  readonly pendingFill = this._pendingFill.asReadonly();

  readonly mode = computed(() => paletteMode(this._text()));

  /** Nothing typed and nothing chosen: the recent things of every kind. */
  readonly recent = computed(() => {
    const mode = this.mode();
    return mode.kind === 'search' && mode.term === '' && this._chips().length === 0;
  });

  /** A space or a tag narrows to notes; no chip at all is every kind. */
  readonly searched = computed<ReadonlySet<PaletteKind>>(() => {
    const chips = this._chips();
    const kinds = new Set(chips.flatMap((chip) => (chip.kind === 'scope' ? [chip.scope] : [])));
    if (chips.some((chip) => chip.kind !== 'scope')) kinds.add('notes');
    return kinds.size === 0 ? new Set(PALETTE_KINDS) : kinds;
  });

  private asked = 0;

  private readonly scheduleSearch = debounced<void>(() => void this.search(), SEARCH_DEBOUNCE_MS);

  /** Opens on the recent things: reopening without typing has a meaning. */
  async open(fromElsewhere: boolean): Promise<void> {
    this._isOpen.set(true);
    this._fromElsewhere.set(fromElsewhere);
    this._text.set('');
    this._chips.set([]);
    this._pendingFill.set(null);
    await Promise.all([this.search(), this.loadTags()]);
  }

  close(): void {
    this.scheduleSearch.cancel();
    this._isOpen.set(false);
    this._pendingFill.set(null);
  }

  setText(text: string): void {
    this._text.set(text);
    this.scheduleSearch();
  }

  /** The word that asked for it leaves the field. A second space takes the place of the first. */
  addChip(chip: PaletteChip): void {
    const kept = this._chips().filter(
      (other) => !samePlace(other, chip) && !(chip.kind === 'space' && other.kind === 'space'),
    );
    this._chips.set([...kept, chip]);
    this._text.set(
      this._text()
        .replace(/(?:^|\s)[@#]\S*$/, '')
        .trimEnd(),
    );
    void this.search();
  }

  removeChip(index: number): void {
    this._chips.set(this._chips().filter((_, at) => at !== index));
    void this.search();
  }

  /** A snippet with fields goes through the form first, or the copy is unusable. */
  /** A drawing is no text: it is exported from its editor, not copied. */
  async copy(note: Note): Promise<void> {
    if (note.kind === 'drawing') return;
    if (note.placeholders.length > 0) {
      this._pendingFill.set(note);
      return;
    }

    const whole = await this.notifier.attempt('errors.noteReadFailed', () =>
      this.notesRepository.whole(note),
    );
    if (!whole) return;
    // A todo list has no content, and a sheet copies with its results.
    const text = whole.kind === 'calc' ? await this.calc.copyText(whole.content) : noteCopyText(whole);
    if (text !== null) await this.copyAndDismiss(text, note.title);
  }

  /**
   * Over another application the window goes back where it came from, and the toast, sent
   * once it has gone, is the only acknowledgement: `StatusNotifier` draws under the titlebar.
   * With DevNotes already in front it only closes, and says so where it stays.
   */
  async copyAndDismiss(content: string, title: string): Promise<void> {
    if (!(await this.clipboard.copy(content))) {
      this.notifier.notify({ ref: { key: 'errors.copyFailed' } });
      return;
    }

    const fromElsewhere = this._fromElsewhere();
    this.close();
    if (!fromElsewhere) {
      this.status.notify(
        title ? { key: 'palette.copiedNote', params: { title } } : { key: 'palette.copiedUntitled' },
      );
      return;
    }

    await this.window.hide();
    await this.desktop.notify(
      { key: 'palette.copiedTitle' },
      title ? { key: 'palette.copiedNote', params: { title } } : { key: 'palette.copiedUntitled' },
    );
  }

  /** « Copier tel quel » from the fields form: the whole body, the result holding a preview. */
  async copyRaw(note: Note): Promise<void> {
    const whole = await this.notifier.attempt('errors.noteReadFailed', () =>
      this.notesRepository.whole(note),
    );
    if (whole) await this.copyAndDismiss(whole.content, whole.title);
  }

  cancelFill(): void {
    this._pendingFill.set(null);
  }

  /** The last one asked wins: an answer to an older search is dropped. */
  private async search(): Promise<void> {
    const asked = ++this.asked;
    const mode = this.mode();
    const term = mode.kind === 'search' ? mode.term : '';
    const searched = this.searched();
    const wanted = mode.kind === 'search';

    const [notes, requests] = await Promise.all([
      wanted && searched.has('notes') ? this.queryNotes(term) : [],
      wanted && searched.has('requests')
        ? this.notifier.attempt('http.failed', () => this.httpRepository.search(term))
        : [],
    ]);
    if (asked !== this.asked) return;

    this._notes.set(notes ?? []);
    this._requests.set(requests ?? []);
  }

  private async queryNotes(term: string): Promise<readonly Note[]> {
    const now = this.clock.now();
    const chips = this._chips();
    const space = chips.find((chip) => chip.kind === 'space');
    const view = await this.notifier.attempt('errors.notesLoadFailed', () =>
      this.notesRepository.query({
        spaceId: space?.kind === 'space' ? space.id : null,
        folderId: null,
        search: term,
        filter: 'all',
        tags: chips.flatMap((chip) => (chip.kind === 'tag' ? [chip.tag] : [])),
        languages: [],
        kinds: [],
        priorities: [],
        now,
        tzOffsetMinutes: now.getTimezoneOffset(),
        // The one place the pinned hoist is a setting.
        pinnedFirst: this.settings.showPinnedFirst(),
        ranked: true,
      }),
    );
    return view ? view.sections.flatMap((section) => [...section.notes]).slice(0, MAX_NOTES) : [];
  }

  private async loadTags(): Promise<void> {
    const tags = await this.notifier.attempt('errors.notesLoadFailed', () => this.notesRepository.loadTags());
    this._tags.set(tags ?? []);
  }
}

function samePlace(one: PaletteChip, other: PaletteChip): boolean {
  if (one.kind === 'scope' && other.kind === 'scope') return one.scope === other.scope;
  if (one.kind === 'tag' && other.kind === 'tag') return one.tag === other.tag;
  return false;
}
