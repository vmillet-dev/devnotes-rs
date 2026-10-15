import {
  ChangeDetectionStrategy,
  Component,
  DestroyRef,
  ElementRef,
  Injector,
  afterNextRender,
  effect,
  inject,
  input,
  output,
  signal,
  untracked,
  viewChild,
} from '@angular/core';
import { toSignal } from '@angular/core/rxjs-interop';
import { TranslocoService } from '@jsverse/transloco';
import {
  defaultKeymap,
  history,
  historyKeymap,
  isolateHistory,
  undo as undoEdit,
} from '@codemirror/commands';
import { bracketMatching, indentUnit, syntaxHighlighting } from '@codemirror/language';
import { highlightSelectionMatches, search, searchKeymap } from '@codemirror/search';
import { Annotation, Compartment, EditorState, Extension, Transaction } from '@codemirror/state';
import {
  EditorView,
  KeyBinding,
  ViewUpdate,
  drawSelection,
  highlightSpecialChars,
  keymap,
  lineNumbers,
  placeholder,
} from '@codemirror/view';
import type { Span } from '@core/model/json.model';
import type { LanguageTag } from '@core/model/language.model';
import { PreferencesService } from '@core/services/preferences/preferences.service';
import type { CodeSelection } from '../code-selection';
import { rewrite } from '../indentation';
import { TAB_KEYS, applyEdit } from './editing';
import { hangingIndent } from './hanging-indent';
import { languageOf } from './languages';
import { lineMarks, setLineMarks } from './line-marks';
import { phrasesFrom } from './phrases';
import { searchPanel } from './search-panel';
import { openReplace, readSearchOptions } from './search-query';
import { CODE_COLOURS, CODE_THEME } from './theme';

const NO_LINES: ReadonlySet<number> = new Set();

/** Ctrl+H, as VS Code binds it; from the bar too, which reaches the keymap by its scope. */
const REPLACE_KEYS: readonly KeyBinding[] = [
  { key: 'Mod-h', run: openReplace, scope: 'editor search-panel', preventDefault: true },
];

const WRAPPED: Extension = [EditorView.lineWrapping, hangingIndent];

/** What came in through `content`: the overlay holds it already, it is not handed back. */
const FROM_OUTSIDE = Annotation.define<boolean>();

/**
 * A snippet's code in CodeMirror 6: only the lines on screen are drawn, and the grammar parses
 * as it goes, so a 2 MB document types like a short one. The draft stays the overlay's — this
 * hands every edit out, and takes in what changes the text from outside (a restored revision, a
 * format on save) as one undoable step.
 */
@Component({
  selector: 'app-code-editor',
  template: '<div #host class="code-editor" data-testid="editor-body"></div>',
  styleUrl: './code-editor.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class CodeEditorComponent implements CodeSelection {
  /** Another one is another note: a fresh history, Ctrl+Z never reaches into the previous. */
  readonly session = input.required<number>();
  readonly content = input.required<string>();
  readonly language = input.required<LanguageTag>();
  readonly indentUnit = input.required<string>();
  readonly wrap = input(false);
  readonly markedLines = input<ReadonlySet<number>>(NO_LINES);
  readonly focusedLines = input<ReadonlySet<number>>(NO_LINES);
  readonly label = input('');
  readonly placeholder = input('');

  readonly changed = output<string>();
  /** The focus left the editor, its search panel included. */
  readonly blurred = output<void>();
  /** Where a key or a click put the caret; never what `reveal` selects. */
  readonly caret = output<number>();
  readonly pasted = output<ClipboardEvent>();

  private readonly host = viewChild.required<ElementRef<HTMLElement>>('host');
  private readonly injector = inject(Injector);
  private readonly preferences = inject(PreferencesService);
  private readonly view = signal<EditorView | null>(null);
  private readonly words = toSignal(
    inject(TranslocoService).selectTranslateObject<Record<string, unknown>>('editor.code'),
    { initialValue: {} },
  );

  private readonly grammar = new Compartment();
  private readonly unit = new Compartment();
  private readonly wrapping = new Compartment();
  private readonly phrases = new Compartment();
  private readonly attributes = new Compartment();
  private readonly hint = new Compartment();
  private loadedGrammar: Extension = [];
  private openedSession = 0;
  /** The text last handed out: the same string coming back in is not an edit from outside. */
  private emitted: string | null = null;

  constructor() {
    afterNextRender({ write: () => this.view.set(this.create()) });
    inject(DestroyRef).onDestroy(() => this.view()?.destroy());

    effect(() => {
      const view = this.view();
      const session = this.session();
      const content = this.content();
      if (view) untracked(() => this.follow(view, session, content));
    });

    effect(() => {
      const view = this.view();
      const tag = this.language();
      if (!view) return;
      void languageOf(tag).then((grammar) => {
        if (this.view() !== view || this.language() !== tag) return;
        this.loadedGrammar = grammar;
        view.dispatch({ effects: this.grammar.reconfigure(grammar) });
      });
    });

    this.reconfigureOn(this.unit, () => indentUnit.of(this.indentUnit()));
    this.reconfigureOn(this.wrapping, () => (this.wrap() ? WRAPPED : []));
    this.reconfigureOn(this.phrases, () => EditorState.phrases.of(phrasesFrom(this.words())));
    this.reconfigureOn(this.attributes, () =>
      EditorView.contentAttributes.of({ 'aria-label': this.label() }),
    );
    this.reconfigureOn(this.hint, () => placeholder(this.placeholder()));

    effect(() => {
      const view = this.view();
      const marks = { marked: this.markedLines(), focused: this.focusedLines() };
      if (view) view.dispatch({ effects: setLineMarks.of(marks) });
    });
  }

  /** Selects the span and scrolls it a third of the way down; takes the focus only when asked. */
  reveal(span: Span, focus: boolean): void {
    const view = this.view();
    if (!view) return;
    const end = view.state.doc.length;
    const anchor = Math.min(span.start, end);
    view.dispatch({
      selection: { anchor, head: Math.min(span.end, end) },
      effects: EditorView.scrollIntoView(anchor, { y: 'start', yMargin: view.scrollDOM.clientHeight / 3 }),
    });
    if (focus) view.focus();
  }

  /** Prettier's text: only what differs is replaced, in one step the undo takes back whole. */
  applyFormat(text: string, cursor: number): void {
    const view = this.view();
    if (view) applyEdit(view, rewrite(view.state.doc.toString(), text, cursor), 'input.format', true);
  }

  /** The caret at `at`, focused and in sight: where Prettier stopped reading. */
  select(at: number): void {
    const view = this.view();
    if (!view) return;
    const position = Math.min(at, view.state.doc.length);
    view.dispatch({ selection: { anchor: position }, scrollIntoView: true });
    view.focus();
  }

  undo(): void {
    const view = this.view();
    if (!view) return;
    view.focus();
    undoEdit(view);
  }

  hasFocus(): boolean {
    return this.view()?.hasFocus ?? false;
  }

  /** Escape's first step: out of the code, the editor still open. */
  blur(): void {
    this.view()?.contentDOM.blur();
  }

  /** Where the selection starts, as the formatter wants its cursor. */
  caretAt(): number {
    return this.view()?.state.selection.main.from ?? 0;
  }

  private reconfigureOn(slot: Compartment, build: () => Extension): void {
    effect(() => {
      const view = this.view();
      const extension = build();
      if (view) view.dispatch({ effects: slot.reconfigure(extension) });
    });
  }

  private create(): EditorView {
    this.openedSession = untracked(this.session);
    const view = new EditorView({
      parent: this.host().nativeElement,
      state: this.state(untracked(this.content)),
    });

    // After CodeMirror's own handlers: an Escape that closed its search panel, or folded a
    // selection to a caret, must not close the editor too.
    view.dom.addEventListener('keydown', (event) => {
      if (event.key === 'Escape' && event.defaultPrevented) event.stopPropagation();
    });
    view.dom.addEventListener('focusout', (event) => {
      if (!(event.relatedTarget instanceof Node && view.dom.contains(event.relatedTarget)))
        this.blurred.emit();
    });
    return view;
  }

  private state(doc: string): EditorState {
    return EditorState.create({
      doc,
      extensions: [
        lineNumbers(),
        highlightSpecialChars(),
        history(),
        drawSelection(),
        bracketMatching(),
        highlightSelectionMatches(),
        search({
          top: true,
          createPanel: searchPanel(this.injector),
          ...readSearchOptions(this.preferences),
        }),
        syntaxHighlighting(CODE_COLOURS),
        CODE_THEME,
        lineMarks,
        keymap.of([...TAB_KEYS, ...REPLACE_KEYS, ...searchKeymap, ...historyKeymap, ...defaultKeymap]),
        EditorView.updateListener.of((update) => this.onUpdate(update)),
        EditorView.domEventHandlers({
          paste: (event) => {
            this.pasted.emit(event);
            return event.defaultPrevented;
          },
        }),
        untracked(() => [
          this.grammar.of(this.loadedGrammar),
          this.unit.of(indentUnit.of(this.indentUnit())),
          this.wrapping.of(this.wrap() ? WRAPPED : []),
          this.phrases.of(EditorState.phrases.of(phrasesFrom(this.words()))),
          this.attributes.of(EditorView.contentAttributes.of({ 'aria-label': this.label() })),
          this.hint.of(placeholder(this.placeholder())),
        ]),
      ],
    });
  }

  /**
   * ⚠️ CodeMirror joins lines with `\n`: a CRLF draft reads back as LF, and is not an edit until
   * the user makes one — as the textarea's value already did.
   */
  private follow(view: EditorView, session: number, content: string): void {
    if (session !== this.openedSession) {
      this.openedSession = session;
      this.emitted = null;
      view.setState(this.state(content));
      view.dispatch({
        effects: setLineMarks.of({ marked: this.markedLines(), focused: this.focusedLines() }),
      });
      return;
    }
    if (content === this.emitted) return;

    const before = view.state.doc.toString();
    const after = content.replace(/\r\n?/g, '\n');
    if (before === after) return;
    const edit = rewrite(before, after, 0);
    view.dispatch({
      changes: { from: edit.from, to: edit.to, insert: edit.insert },
      annotations: [isolateHistory.of('full'), FROM_OUTSIDE.of(true)],
    });
  }

  private onUpdate(update: ViewUpdate): void {
    if (
      update.transactions.some(
        (transaction) => transaction.docChanged && !transaction.annotation(FROM_OUTSIDE),
      )
    ) {
      this.emitted = update.state.doc.toString();
      this.changed.emit(this.emitted);
    }
    const byUser = update.transactions.some((transaction) => transaction.annotation(Transaction.userEvent));
    if (update.selectionSet && byUser) this.caret.emit(update.state.selection.main.from);
  }
}
