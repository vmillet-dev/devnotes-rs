import {
  ChangeDetectionStrategy,
  Component,
  DestroyRef,
  ElementRef,
  afterNextRender,
  computed,
  effect,
  inject,
  input,
  output,
  signal,
  untracked,
  viewChild,
} from '@angular/core';
import { toSignal } from '@angular/core/rxjs-interop';
import { TranslocoPipe, TranslocoService } from '@jsverse/transloco';
import {
  CompletionContext,
  CompletionResult,
  acceptCompletion,
  autocompletion,
  completionKeymap,
} from '@codemirror/autocomplete';
import { defaultKeymap, history, historyKeymap, isolateHistory } from '@codemirror/commands';
import { Annotation, EditorState, Prec } from '@codemirror/state';
import {
  EditorView,
  ViewUpdate,
  drawSelection,
  hoverTooltip,
  keymap,
  lineNumbers,
  placeholder,
} from '@codemirror/view';
import {
  CALC_DECIMALS,
  CalcDecimals,
  CalcSheet,
  CalcValue,
  CalcVariable,
  unitsOf,
} from '@core/model/calc.model';
import { CalcSheets } from '@core/services/calc/calc-sheets.service';
import { decimalMean, decimalSum } from '@core/services/calc/calc-format';
import { ClipboardService } from '@core/services/clipboard/clipboard.service';
import { ErrorNotifier } from '@core/services/errors/error-notifier.service';
import { SettingsStore } from '@core/services/settings/settings.store';
import { debounced } from '@core/services/time/debounce';
import { rewrite } from '../indentation';
import { CODE_THEME } from '../code-editor/theme';
import { CALC_THEME } from './calc-theme';
import { ShownResult, marksFor, setSheetMarks, sheetMarks } from './sheet-marks';

/** Fast enough to read as live, slow enough not to ask Rust at every key. */
const EVALUATE_AFTER_MS = 120;
/** How long « Copié » stays in the footer before the sum comes back. */
const SAID_FOR_MS = 1800;

/** What came in through `content`: the overlay holds it already, it is not handed back. */
const FROM_OUTSIDE = Annotation.define<boolean>();

/** `en`, `in`, `à`… then the start of a unit: what the units of the line's kind complete. */
const CONVERSION = /(?:^|\s)(?:en|in|to|as|vers|à|at)\s+([^\s]*)$/i;

/** A result's copy icon or its value, under a press or a click. */
function control(event: MouseEvent): HTMLElement | null {
  const target = event.target instanceof Element ? event.target : null;
  return target?.closest<HTMLElement>('[data-copy], [data-pick]') ?? null;
}

/** A caret moving inside one line selects no line, and draws nothing again. */
function sameLines(a: ReadonlySet<number>, b: ReadonlySet<number>): boolean {
  return a.size === b.size && [...a].every((line) => b.has(line));
}

interface Answer {
  readonly text: string;
  readonly sheet: CalcSheet;
}

interface Summary {
  readonly count: number;
  readonly sum: string | null;
  readonly mean: string | null;
  readonly problem: 'nothingToAdd' | 'mixedUnits' | null;
}

/**
 * A Calcul note: the sheet on the left in CodeMirror, each line's result on the right as Rust
 * computes it, written in the language on screen. The draft stays the overlay's, as the code
 * field's does: every edit is handed out, and a change from outside is one undoable step.
 */
@Component({
  selector: 'app-calc-editor',
  imports: [TranslocoPipe],
  templateUrl: './calc-editor.component.html',
  styleUrl: './calc-editor.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class CalcEditorComponent {
  readonly session = input.required<number>();
  readonly content = input.required<string>();
  readonly label = input('');
  readonly placeholder = input('');

  readonly changed = output<string>();
  readonly blurred = output<void>();

  private readonly host = viewChild.required<ElementRef<HTMLElement>>('host');
  private readonly sheets = inject(CalcSheets);
  private readonly settings = inject(SettingsStore);
  private readonly clipboard = inject(ClipboardService);
  private readonly notifier = inject(ErrorNotifier);
  private readonly transloco = inject(TranslocoService);
  private readonly lang = toSignal(this.transloco.langChanges$, {
    initialValue: this.transloco.getActiveLang(),
  });

  private readonly view = signal<EditorView | null>(null);
  private readonly text = signal('');
  protected readonly answer = signal<Answer | null>(null);
  /** The results clicked: Shift takes a range, Ctrl one more. Zero-based lines. */
  protected readonly picked = signal<ReadonlySet<number>>(new Set());
  /** The lines a selection of the text covers, when it spans more than a caret. */
  private readonly selectedLines = signal<ReadonlySet<number>>(new Set(), { equal: sameLines });
  protected readonly said = signal<string | null>(null);
  private anchor = 0;
  private openedSession = 0;
  private emitted: string | null = null;
  private asked = 0;

  protected readonly decimalChoices = CALC_DECIMALS;
  protected readonly decimals = this.settings.calcDecimals;

  protected readonly formatter = computed(() => {
    this.lang();
    return this.sheets.formatter(this.decimals());
  });

  protected readonly lineCount = computed(() => this.text().split('\n').length);
  protected readonly variableCount = computed(
    () => new Set((this.answer()?.sheet.variables ?? []).map((variable) => variable.name)).size,
  );

  /** The text's selection when it spans lines, the clicked results otherwise. */
  private readonly chosen = computed(() =>
    this.selectedLines().size > 0 ? this.selectedLines() : this.picked(),
  );

  protected readonly summary = computed<Summary | null>(() => {
    const lines = [...this.chosen()].sort((a, b) => a - b);
    const rows = this.answer()?.sheet.rows ?? [];
    if (lines.length === 0) return null;
    const values = lines.flatMap((line) => {
      const row = rows[line]?.line;
      return row?.kind === 'value' && row.value.kind === 'number' ? [row.value] : [];
    });
    if (values.length === 0) return { count: lines.length, sum: null, mean: null, problem: 'nothingToAdd' };
    const units = new Set(values.map((value) => JSON.stringify(value.unit)));
    if (units.size > 1) return { count: lines.length, sum: null, mean: null, problem: 'mixedUnits' };
    const unit = values[0]!.unit;
    const shown = (value: string): string => {
      const text = this.formatter().value({ kind: 'number', value, unit });
      return `${text.approx ? '≈ ' : ''}${text.text}`;
    };
    const exact = values.map((value) => value.value);
    return {
      count: lines.length,
      sum: shown(decimalSum(exact)),
      mean: shown(decimalMean(exact)),
      problem: null,
    };
  });

  private readonly schedule = debounced<string>((text) => void this.evaluate(text), EVALUATE_AFTER_MS);
  private readonly unsay = debounced<void>(() => this.said.set(null), SAID_FOR_MS);

  constructor() {
    afterNextRender({ write: () => this.view.set(this.create()) });
    inject(DestroyRef).onDestroy(() => this.view()?.destroy());

    effect(() => {
      const view = this.view();
      const session = this.session();
      const content = this.content();
      if (view) untracked(() => this.follow(view, session, content));
    });

    // The results are drawn again when they change, when the language or the decimals do,
    // and when one is picked: only for the text they were computed from.
    effect(() => {
      const view = this.view();
      const answer = this.answer();
      const formatter = this.formatter();
      const picked = this.chosen();
      if (!view || !answer || view.state.doc.toString() !== answer.text) return;
      const results = this.results(answer.sheet, formatter, picked);
      untracked(() =>
        view.dispatch({ effects: setSheetMarks.of(marksFor(view.state.doc, answer.sheet, results)) }),
      );
    });
  }

  protected onDecimals(event: Event): void {
    const value = (event.target as HTMLSelectElement).value;
    const choice = CALC_DECIMALS.find((option) => option === value);
    if (choice) this.settings.calcDecimals.write(choice);
  }

  protected decimalsLabel(choice: CalcDecimals): string {
    return choice === 'auto' ? this.transloco.translate('calc.editor.decimalsAuto') : choice;
  }

  private results(
    sheet: CalcSheet,
    formatter: ReturnType<CalcSheets['formatter']>,
    picked: ReadonlySet<number>,
  ): ShownResult[] {
    return sheet.rows.flatMap((row, line): ShownResult[] => {
      if (row.line.kind === 'value') {
        const shown = formatter.value(row.line.value);
        const exact = formatter.exact(row.line.value);
        return [
          {
            line,
            text: shown.text,
            approx: shown.approx,
            error: false,
            named: row.line.name !== null,
            selected: picked.has(line),
            copyLabel: this.transloco.translate('calc.editor.copyResult', { value: exact }),
            pickLabel: this.transloco.translate('calc.editor.pick'),
          },
        ];
      }
      if (row.line.kind === 'error') {
        return [
          {
            line,
            text: formatter.error(row.line.error),
            approx: false,
            error: true,
            named: false,
            selected: false,
            copyLabel: '',
            pickLabel: '',
          },
        ];
      }
      return [];
    });
  }

  private async evaluate(text: string): Promise<void> {
    const asked = ++this.asked;
    const sheet = await this.notifier.attempt('errors.calcFailed', () => this.sheets.evaluate(text));
    if (asked !== this.asked || sheet === null) return;
    this.answer.set({ text, sheet });
  }

  private valueAt(line: number): CalcValue | null {
    const row = this.answer()?.sheet.rows[line]?.line;
    return row?.kind === 'value' ? row.value : null;
  }

  /** A click picks one, Shift a range from the last one picked, Ctrl one more or one less. */
  private pick(line: number, event: MouseEvent): void {
    const current = this.picked();
    let next: Set<number>;
    if (event.shiftKey) {
      next = new Set();
      const [from, to] = [Math.min(this.anchor, line), Math.max(this.anchor, line)];
      for (let at = from; at <= to; at += 1) if (this.valueAt(at)) next.add(at);
    } else if (event.ctrlKey || event.metaKey) {
      next = new Set(current);
      if (!next.delete(line)) next.add(line);
      this.anchor = line;
    } else {
      next = current.size === 1 && current.has(line) ? new Set() : new Set([line]);
      this.anchor = line;
    }
    this.picked.set(next);
  }

  private async copy(line: number): Promise<void> {
    const value = this.valueAt(line);
    if (!value) return;
    const exact = this.formatter().exact(value);
    if (!(await this.clipboard.copy(exact))) {
      this.notifier.notify({ ref: { key: 'errors.copyFailed' } });
      return;
    }
    this.said.set(this.transloco.translate('calc.editor.copied', { value: exact }));
    this.unsay();
  }

  private create(): EditorView {
    this.openedSession = untracked(this.session);
    const view = new EditorView({
      parent: this.host().nativeElement,
      state: this.state(untracked(this.content)),
    });
    this.text.set(view.state.doc.toString());
    void this.evaluate(view.state.doc.toString());

    // After CodeMirror's own handlers: an Escape that closed the completion must not close
    // the editor too.
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
        history(),
        drawSelection(),
        CODE_THEME,
        CALC_THEME,
        sheetMarks,
        autocompletion({ override: [(context) => this.complete(context)], icons: false }),
        hoverTooltip((view, position) => this.tooltip(view, position)),
        Prec.highest(keymap.of([{ key: 'Tab', run: acceptCompletion }])),
        keymap.of([...completionKeymap, ...historyKeymap, ...defaultKeymap]),
        EditorView.updateListener.of((update) => this.onUpdate(update)),
        EditorView.domEventHandlers({
          mousedown: (event) => this.onMouseDown(event),
          click: (event) => this.onClick(event),
        }),
        EditorView.contentAttributes.of({ 'aria-label': untracked(this.label) }),
        placeholder(untracked(this.placeholder)),
      ],
    });
  }

  /** The icons and the results are no text to put the caret in: their clicks stop here. */
  private onMouseDown(event: MouseEvent): boolean {
    if (!control(event)) return false;
    event.preventDefault();
    return true;
  }

  /** The click acts, the press before it only kept the caret where it was. */
  private onClick(event: MouseEvent): boolean {
    const target = control(event);
    if (!target) return false;
    event.preventDefault();
    if (target.dataset['copy'] !== undefined) void this.copy(Number(target.dataset['copy']));
    else this.pick(Number(target.dataset['pick']), event);
    return true;
  }

  /** After `en` or `à`, only the units of the line's kind, as Rust read it a moment ago. */
  private async complete(context: CompletionContext): Promise<CompletionResult | null> {
    const line = context.state.doc.lineAt(context.pos);
    const before = line.text.slice(0, context.pos - line.from);
    const match = CONVERSION.exec(before);
    if (!match) return null;
    const sheet = await this.sheets.evaluate(context.state.doc.toString());
    const dimension = sheet.rows[line.number - 1]?.converting;
    if (!dimension) return null;
    const formatter = this.formatter();
    const options = unitsOf(dimension).map((unit) => ({
      label: formatter.symbol(unit),
      detail: this.transloco.translate(`calc.unitNames.${unit}`),
      type: 'unit',
    }));
    if (dimension === 'number') options.push({ label: '%', detail: '', type: 'unit' });
    return { from: context.pos - (match[1]?.length ?? 0), options, validFor: /^[^\s]*$/ };
  }

  /** A name hovered says its exact value, where it was given and where it is read. */
  private tooltip(view: EditorView, position: number) {
    const answer = this.answer();
    if (!answer || view.state.doc.toString() !== answer.text) return null;
    const line = view.state.doc.lineAt(position);
    const index = line.number - 1;
    const span = answer.sheet.rows[index]?.spans.find(
      (candidate) =>
        candidate.kind === 'name' &&
        line.from + candidate.start <= position &&
        position <= line.from + candidate.end,
    );
    if (!span) return null;
    const name = line.text.slice(span.start, span.end);
    const variable = this.definitionOf(answer.sheet.variables, name, index);
    if (!variable) return null;
    return {
      pos: line.from + span.start,
      end: line.from + span.end,
      above: true,
      create: () => {
        const dom = document.createElement('div');
        dom.className = 'calc-tooltip';
        dom.dataset['testid'] = 'calc-variable';
        const value = document.createElement('strong');
        value.textContent = `${variable.name} = ${this.formatter().exact(variable.value)}`;
        const where = document.createElement('span');
        where.textContent = this.whereOf(variable);
        dom.append(value, where);
        return { dom };
      },
    };
  }

  /** The definition in effect on a line: the last one given at or above it. */
  private definitionOf(variables: readonly CalcVariable[], name: string, line: number): CalcVariable | null {
    return variables.filter((variable) => variable.name === name && variable.line <= line).at(-1) ?? null;
  }

  private whereOf(variable: CalcVariable): string {
    const defined = this.transloco.translate('calc.editor.defined', { line: variable.line + 1 });
    const lines = variable.uses.map((use) => use + 1);
    const used =
      lines.length === 0
        ? this.transloco.translate('calc.editor.unused')
        : lines.length === 1
          ? this.transloco.translate('calc.editor.usedOnce', { line: lines[0] })
          : this.transloco.translate('calc.editor.usedMany', { lines: lines.join(', ') });
    return `${defined} · ${used}`;
  }

  /**
   * ⚠️ CodeMirror joins lines with `\n`: a CRLF draft reads back as LF, and is not an edit until
   * the user makes one.
   */
  private follow(view: EditorView, session: number, content: string): void {
    if (session !== this.openedSession) {
      this.openedSession = session;
      this.emitted = null;
      this.picked.set(new Set());
      view.setState(this.state(content));
      this.text.set(view.state.doc.toString());
      void this.evaluate(view.state.doc.toString());
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
    if (update.docChanged) {
      const text = update.state.doc.toString();
      this.text.set(text);
      this.schedule(text);
      if (
        update.transactions.some(
          (transaction) => transaction.docChanged && !transaction.annotation(FROM_OUTSIDE),
        )
      ) {
        this.emitted = text;
        this.changed.emit(text);
      }
    }
    if (update.selectionSet || update.docChanged) {
      const { from, to } = update.state.selection.main;
      const doc = update.state.doc;
      const lines = new Set<number>();
      if (from !== to) {
        const [first, last] = [doc.lineAt(from).number, doc.lineAt(to).number];
        for (let line = first; line <= last; line += 1) lines.add(line - 1);
      }
      this.selectedLines.set(lines);
    }
  }
}
