import {
  ChangeDetectionStrategy,
  Component,
  DestroyRef,
  ElementRef,
  Injector,
  afterNextRender,
  computed,
  inject,
  input,
  signal,
  viewChild,
} from '@angular/core';
import { TranslocoPipe, TranslocoService } from '@jsverse/transloco';
import { PreferencesService } from '@core/services/preferences/preferences.service';
import { IconComponent } from '@shared/icon/icon.component';
import { SearchOption, readSearchOptions, writeSearchOptions } from '../search-options';
import { MatchCount, SearchChange, SearchSpec, SearchTarget } from '../search-target';

/**
 * Past this length, finding and counting wait for a pause in the typing: on 2 MB, a word found
 * nowhere means reading the whole text, some 150 ms a key. The highlighting, on screen only, does not.
 */
const LARGE = 200_000;
const PAUSE_MS = 120;

const NONE: MatchCount = { total: 0, current: null, capped: false };

/**
 * One search bar over either editor: it draws the query and keeps the count, the editor behind
 * `SearchTarget` finds, highlights and replaces. Typing selects the first match from where the
 * search started, as a browser does.
 */
@Component({
  selector: 'app-search-bar',
  imports: [IconComponent, TranslocoPipe],
  templateUrl: './search-bar.component.html',
  styleUrl: './search-bar.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
  host: { role: 'search', '(keydown)': 'onKeydown($event)' },
})
export class SearchBarComponent {
  readonly target = input.required<SearchTarget>();

  private readonly preferences = inject(PreferencesService);
  private readonly transloco = inject(TranslocoService);
  private readonly injector = inject(Injector);
  private readonly findField = viewChild.required<ElementRef<HTMLInputElement>>('findField');
  private readonly replaceField = viewChild<ElementRef<HTMLInputElement>>('replaceField');

  protected readonly text = signal('');
  protected readonly replaceText = signal('');
  protected readonly options = signal(readSearchOptions(this.preferences));
  protected readonly replaceOpen = signal(false);
  protected readonly count = signal<MatchCount>(NONE);
  /** A large text's count, waiting for the pause: neither a number nor 'nothing' yet. */
  protected readonly counting = signal(false);

  protected readonly nothingFound = computed(
    () => this.text() !== '' && !this.counting() && this.count().total === 0,
  );

  /** `3/12`, `12` while the selection is on none, `1 000+` past the cap. */
  protected readonly countText = computed(() => {
    const { total, current, capped } = this.count();
    if (this.text() === '') return '';
    if (this.counting()) return '…';
    const number = new Intl.NumberFormat(this.transloco.getActiveLang());
    const all = `${number.format(total)}${capped ? '+' : ''}`;
    return current === null ? all : `${number.format(current)}/${all}`;
  });

  /** Where the typing searches from: the caret when the bar opened. */
  private origin = 0;
  private readonly timers = new Map<'seek' | 'count', ReturnType<typeof setTimeout>>();

  constructor() {
    inject(DestroyRef).onDestroy(() => this.timers.forEach((timer) => clearTimeout(timer)));
  }

  /** The editor opened the bar, the query already taken from the selection. */
  mounted(): void {
    const target = this.target();
    this.origin = target.caret();
    this.adopt(target.query());
    this.recount();
    const field = this.findField().nativeElement;
    field.focus();
    field.select();
  }

  /** Every editor update: a query set from outside (Ctrl+F on a selection), an edit, a move. */
  follow(change: SearchChange): void {
    if (change.query) this.adopt(change.query);
    if (change.replace) this.unfoldReplace();
    if (change.query || change.edited) this.recount();
  }

  protected onFind(value: string): void {
    this.text.set(value);
    this.search();
  }

  protected onReplaceInput(value: string): void {
    this.replaceText.set(value);
    this.target().setQuery(this.query());
  }

  protected toggle(option: SearchOption): void {
    this.options.update((options) => ({ ...options, [option]: !options[option] }));
    writeSearchOptions(this.preferences, this.options());
    this.search();
  }

  protected toggleReplace(): void {
    if (this.replaceOpen()) {
      this.replaceOpen.set(false);
      this.findField().nativeElement.focus();
    } else {
      this.unfoldReplace();
    }
  }

  protected next(): void {
    this.target().next();
  }

  protected previous(): void {
    this.target().previous();
  }

  protected replaceOne(): void {
    this.target().replaceOne();
  }

  protected replaceEvery(): void {
    this.target().replaceAll();
  }

  protected close(): void {
    this.target().close();
  }

  /**
   * Enter and Shift+Enter move between matches, in the replace field Enter replaces; F3, Ctrl+G
   * Ctrl+H and Escape are the editor's, reached from here as from inside it: Escape closes the
   * bar and hands the caret back, prevented so that the editor stops it there.
   */
  protected onKeydown(event: KeyboardEvent): void {
    const target = this.target();
    if (target.handleKey(event)) {
      event.preventDefault();
      return;
    }
    if (event.key !== 'Enter') return;
    event.preventDefault();
    if (event.target === this.replaceField()?.nativeElement) {
      target.replaceOne();
    } else if (event.shiftKey) {
      target.previous();
    } else {
      target.next();
    }
  }

  private query(): SearchSpec {
    return { search: this.text(), replace: this.replaceText(), ...this.options() };
  }

  private search(): void {
    const query = this.query();
    this.target().setQuery(query);
    this.afterPause('seek', () => this.target().seek(query, this.origin));
  }

  private adopt(query: SearchSpec): void {
    this.text.set(query.search);
    this.replaceText.set(query.replace);
    this.options.set({
      caseSensitive: query.caseSensitive,
      wholeWord: query.wholeWord,
      regexp: query.regexp,
    });
  }

  private unfoldReplace(): void {
    this.replaceOpen.set(true);
    afterNextRender(() => this.replaceField()?.nativeElement.focus(), { injector: this.injector });
  }

  private recount(): void {
    this.counting.set(this.target().size() > LARGE);
    this.afterPause('count', () => {
      this.count.set(this.target().count());
      this.counting.set(false);
    });
  }

  private afterPause(job: 'seek' | 'count', run: () => void): void {
    clearTimeout(this.timers.get(job));
    if (this.target().size() > LARGE) {
      this.timers.set(job, setTimeout(run, PAUSE_MS));
    } else {
      run();
    }
  }
}
