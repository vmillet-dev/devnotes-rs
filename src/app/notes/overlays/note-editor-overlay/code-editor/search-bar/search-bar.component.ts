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
import {
  SearchQuery,
  closeSearchPanel,
  findNext,
  findPrevious,
  getSearchQuery,
  replaceAll,
  replaceNext,
  setSearchQuery,
} from '@codemirror/search';
import { EditorView, ViewUpdate, runScopeHandlers } from '@codemirror/view';
import { PreferencesService } from '@core/services/preferences/preferences.service';
import { IconComponent } from '@shared/icon/icon.component';
import {
  MatchCount,
  SearchOption,
  countMatches,
  firstMatchFrom,
  readSearchOptions,
  showReplace,
  writeSearchOptions,
} from '../search-query';

/**
 * Past this length, finding and counting wait for a pause in the typing: on 2 MB, a word found
 * nowhere means reading the whole text, some 150 ms a key. The highlighting, on screen only, does not.
 */
const LARGE = 200_000;
const PAUSE_MS = 120;

const NONE: MatchCount = { total: 0, current: null, capped: false };

/**
 * Our search bar over CodeMirror's engine: the query, the highlighting and the replacing stay
 * `@codemirror/search`'s; this draws them and keeps the count. Typing selects the first match
 * from where the search started, as a browser does.
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
  readonly view = input.required<EditorView>();

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

  /** CodeMirror opened the bar, the query already taken from the selection. */
  mounted(): void {
    const view = this.view();
    this.origin = view.state.selection.main.from;
    this.adopt(getSearchQuery(view.state));
    this.recount(view);
    const field = this.findField().nativeElement;
    field.focus();
    field.select();
  }

  /** Every editor update: a query set from outside (Ctrl+F on a selection), an edit, a move. */
  follow(update: ViewUpdate): void {
    let queried = false;
    for (const transaction of update.transactions) {
      for (const effect of transaction.effects) {
        if (effect.is(setSearchQuery)) {
          queried = true;
          this.adopt(effect.value);
        }
        if (effect.is(showReplace)) this.unfoldReplace();
      }
    }
    if (queried || update.docChanged || update.selectionSet) this.recount(update.view);
  }

  protected onFind(value: string): void {
    this.text.set(value);
    this.search();
  }

  protected onReplaceInput(value: string): void {
    this.replaceText.set(value);
    this.view().dispatch({ effects: setSearchQuery.of(this.query()) });
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
    findNext(this.view());
  }

  protected previous(): void {
    findPrevious(this.view());
  }

  protected replaceOne(): void {
    replaceNext(this.view());
  }

  protected replaceEvery(): void {
    replaceAll(this.view());
  }

  protected close(): void {
    closeSearchPanel(this.view());
  }

  /**
   * Enter and Shift+Enter move between matches, in the replace field Enter replaces; F3, Ctrl+G
   * Ctrl+H and Escape are the editor's keymap, reached from here as from CodeMirror's own panel:
   * Escape closes the bar and hands the caret back, prevented so that the editor stops it there.
   */
  protected onKeydown(event: KeyboardEvent): void {
    const view = this.view();
    if (runScopeHandlers(view, event, 'search-panel')) {
      event.preventDefault();
      return;
    }
    if (event.key !== 'Enter') return;
    event.preventDefault();
    if (event.target === this.replaceField()?.nativeElement) {
      replaceNext(view);
    } else if (event.shiftKey) {
      findPrevious(view);
    } else {
      findNext(view);
    }
  }

  private query(): SearchQuery {
    return new SearchQuery({ search: this.text(), replace: this.replaceText(), ...this.options() });
  }

  private search(): void {
    const view = this.view();
    const query = this.query();
    view.dispatch({ effects: setSearchQuery.of(query) });
    this.afterPause(view, 'seek', () => this.seek(query));
  }

  /** The first match from the origin, selected and brought into sight. */
  private seek(query: SearchQuery): void {
    const view = this.view();
    const match = firstMatchFrom(query, view.state, this.origin);
    if (!match) return;
    view.dispatch({
      selection: { anchor: match.from, head: match.to },
      effects: EditorView.scrollIntoView(match.from, { y: 'center' }),
      userEvent: 'select.search',
    });
  }

  private adopt(query: SearchQuery): void {
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

  private recount(view: EditorView): void {
    this.counting.set(view.state.doc.length > LARGE);
    this.afterPause(view, 'count', () => {
      this.count.set(countMatches(getSearchQuery(view.state), view.state));
      this.counting.set(false);
    });
  }

  private afterPause(view: EditorView, job: 'seek' | 'count', run: () => void): void {
    clearTimeout(this.timers.get(job));
    if (view.state.doc.length > LARGE) {
      this.timers.set(job, setTimeout(run, PAUSE_MS));
    } else {
      run();
    }
  }
}
