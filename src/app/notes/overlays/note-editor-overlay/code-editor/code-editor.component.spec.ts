import { ComponentFixture, TestBed } from '@angular/core/testing';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { language } from '@codemirror/language';
import { openSearchPanel } from '@codemirror/search';
import { EditorView } from '@codemirror/view';
import { provideTranslocoTesting } from '@testing/provide-transloco-testing';
import { CodeEditorComponent } from './code-editor.component';

describe('CodeEditorComponent', () => {
  let fixture: ComponentFixture<CodeEditorComponent>;
  let changes: string[];
  let carets: number[];
  let blurs: number;

  function view(): EditorView {
    return EditorView.findFromDOM(fixture.nativeElement.querySelector('.cm-editor'))!;
  }

  function doc(): string {
    return view().state.doc.toString();
  }

  async function set(name: string, value: unknown): Promise<void> {
    fixture.componentRef.setInput(name, value);
    await fixture.whenStable();
  }

  /** Another note: its session and its body arrive together, as the overlay's drafts do. */
  async function open(session: number, content: string): Promise<void> {
    fixture.componentRef.setInput('session', session);
    fixture.componentRef.setInput('content', content);
    await fixture.whenStable();
  }

  /** What a keystroke does to the document: a user event, as CodeMirror's input handling sends. */
  function type(insert: string, at = view().state.selection.main.head): void {
    view().dispatch({
      changes: { from: at, insert },
      selection: { anchor: at + insert.length },
      userEvent: 'input.type',
    });
  }

  function key(init: KeyboardEventInit): KeyboardEvent {
    const event = new KeyboardEvent('keydown', { bubbles: true, cancelable: true, ...init });
    view().contentDOM.dispatchEvent(event);
    return event;
  }

  function lineClasses(className: string): number[] {
    return [...fixture.nativeElement.querySelectorAll('.cm-line')].flatMap((line: Element, index: number) =>
      line.classList.contains(className) ? [index] : [],
    );
  }

  async function mount(content: string, extra: Record<string, unknown> = {}): Promise<void> {
    fixture = TestBed.createComponent(CodeEditorComponent);
    const inputs = { session: 1, content, language: 'txt', indentUnit: '  ', ...extra };
    for (const [name, value] of Object.entries(inputs)) fixture.componentRef.setInput(name, value);
    changes = [];
    carets = [];
    blurs = 0;
    fixture.componentInstance.changed.subscribe((text) => changes.push(text));
    fixture.componentInstance.caret.subscribe((at) => carets.push(at));
    fixture.componentInstance.blurred.subscribe(() => blurs++);
    document.body.appendChild(fixture.nativeElement);
    fixture.autoDetectChanges();
    await fixture.whenStable();
  }

  beforeEach(async () => {
    TestBed.configureTestingModule({
      imports: [CodeEditorComponent],
      providers: [provideTranslocoTesting()],
    });
    await mount('first line\nsecond line');
  });

  afterEach(() => fixture.nativeElement.remove());

  it('shows the content with its line numbers, under the address the scenarios use', () => {
    expect(doc()).toBe('first line\nsecond line');
    expect(fixture.nativeElement.querySelector('[data-testid="editor-body"] .cm-editor')).not.toBeNull();
    expect(
      fixture.nativeElement.querySelectorAll('.cm-lineNumbers .cm-gutterElement').length,
    ).toBeGreaterThan(0);
  });

  it('names the field and says what to write in an empty one', async () => {
    await set('label', 'Contenu de la note');
    await set('placeholder', 'Commencer à écrire…');
    expect(view().contentDOM.getAttribute('aria-label')).toBe('Contenu de la note');

    await open(2, '');
    expect(fixture.nativeElement.querySelector('.cm-placeholder')?.textContent).toBe('Commencer à écrire…');
  });

  it('hands every edit out, and where the caret went', () => {
    type('!', 10);

    expect(changes).toEqual(['first line!\nsecond line']);
    expect(carets).toEqual([11]);
  });

  it('takes a text from outside as one step the undo takes back, without handing it out again', async () => {
    await set('content', 'first line\nsecond line, restored');

    expect(doc()).toBe('first line\nsecond line, restored');
    expect(changes).toEqual([]);

    fixture.componentInstance.undo();
    expect(doc()).toBe('first line\nsecond line');
    expect(changes).toEqual(['first line\nsecond line']);
  });

  it('ignores its own edit coming back as the content', async () => {
    type('!', 0);
    await set('content', changes[0]);

    fixture.componentInstance.undo();
    expect(doc()).toBe('first line\nsecond line');
  });

  it('reads a CRLF draft as it is, and changes nothing until it is edited', async () => {
    await set('content', 'a\r\nb');
    expect(doc()).toBe('a\nb');
    expect(changes).toEqual([]);

    await set('content', 'a\nb');
    expect(changes).toEqual([]);
  });

  it('starts a fresh history with another note', async () => {
    type('!', 0);
    await open(2, 'another note');

    fixture.componentInstance.undo();
    expect(doc()).toBe('another note');
  });

  describe('Tab', () => {
    beforeEach(async () => {
      await open(3, 'if ready:\nstart()');
      await set('indentUnit', '    ');
    });

    it('indents to the next level at the caret, and stays in the text', () => {
      view().dispatch({ selection: { anchor: 10 } });
      const event = key({ key: 'Tab' });

      expect(doc()).toBe('if ready:\n    start()');
      expect(event.defaultPrevented).toBe(true);
    });

    it('takes a level back with Shift, and still keeps the key when there is none to take', () => {
      view().dispatch({ selection: { anchor: 10 } });
      key({ key: 'Tab' });
      key({ key: 'Tab', shiftKey: true });
      expect(doc()).toBe('if ready:\nstart()');

      expect(key({ key: 'Tab', shiftKey: true }).defaultPrevented).toBe(true);
      expect(doc()).toBe('if ready:\nstart()');
    });

    it('indents every line a selection touches, with the unit it is given', async () => {
      await set('indentUnit', '\t');
      view().dispatch({ selection: { anchor: 0, head: 17 } });
      key({ key: 'Tab' });

      expect(doc()).toBe('\tif ready:\n\tstart()');
    });
  });

  describe('what the formatter asks of it', () => {
    beforeEach(async () => {
      await open(4, 'f(a,b)');
    });

    it('replaces the text in one step, the caret where Prettier put it', () => {
      view().dispatch({ selection: { anchor: 2 } });
      expect(fixture.componentInstance.caretAt()).toBe(2);

      fixture.componentInstance.applyFormat('f(\n  a,\n  b,\n);', 3);
      expect(doc()).toBe('f(\n  a,\n  b,\n);');
      expect(view().state.selection.main.head).toBe(3);

      fixture.componentInstance.undo();
      expect(doc()).toBe('f(a,b)');
    });

    it('puts the caret where a syntax error is, and the focus with it', () => {
      fixture.componentInstance.select(4);

      expect(view().state.selection.main.head).toBe(4);
      expect(view().hasFocus).toBe(true);
    });

    it('clamps a position past the end', () => {
      fixture.componentInstance.select(99);
      expect(view().state.selection.main.head).toBe(6);
    });
  });

  describe('beside the visualiser', () => {
    it('selects a span without saying the caret moved, and takes the focus only when asked', () => {
      fixture.componentInstance.reveal({ start: 11, end: 17 }, false);

      expect(view().state.sliceDoc(view().state.selection.main.from, view().state.selection.main.to)).toBe(
        'second',
      );
      expect(carets).toEqual([]);
      expect(view().hasFocus).toBe(false);

      fixture.componentInstance.reveal({ start: 0, end: 99 }, true);
      expect(view().state.selection.main.to).toBe(view().state.doc.length);
      expect(view().hasFocus).toBe(true);
    });

    it('lights the lines a format changed and the lines the visualiser selected, until told otherwise', async () => {
      await set('markedLines', new Set([0]));
      await set('focusedLines', new Set([-1, 1, 7]));

      expect(lineClasses('cm-marked')).toEqual([0]);
      expect(lineClasses('cm-focused-line')).toEqual([1]);

      // A line opened between them: each mark keeps to its own line.
      type('\n', 10);
      expect(lineClasses('cm-marked')).toEqual([0]);
      expect(lineClasses('cm-focused-line')).toEqual([2]);

      await set('markedLines', new Set());
      expect(lineClasses('cm-marked')).toEqual([]);
    });
  });

  it('wraps long lines only when asked', async () => {
    expect(view().contentDOM.classList.contains('cm-lineWrapping')).toBe(false);

    await set('wrap', true);
    expect(view().contentDOM.classList.contains('cm-lineWrapping')).toBe(true);
  });

  it("loads the grammar of the snippet's language, and another when it changes", async () => {
    await set('language', 'json');
    await expect.poll(() => view().state.facet(language)?.name).toBe('json');

    await set('language', 'txt');
    await expect.poll(() => view().state.facet(language)).toBeNull();
  });

  it('hands a paste out, and leaves it to CodeMirror unless it was taken', () => {
    const pasted: ClipboardEvent[] = [];
    fixture.componentInstance.pasted.subscribe((event) => {
      pasted.push(event);
      event.preventDefault();
    });
    const event = new Event('paste', { bubbles: true, cancelable: true }) as ClipboardEvent;
    view().contentDOM.dispatchEvent(event);

    expect(pasted).toEqual([event]);
    expect(doc()).toBe('first line\nsecond line');
  });

  describe('Escape', () => {
    let reached: KeyboardEvent[];
    const listen = (event: Event): void => {
      reached.push(event as KeyboardEvent);
    };

    beforeEach(() => {
      reached = [];
      document.addEventListener('keydown', listen);
    });

    afterEach(() => document.removeEventListener('keydown', listen));

    it('goes on to the dialog when CodeMirror has nothing to do with it', () => {
      key({ key: 'Escape' });
      expect(reached).toHaveLength(1);
    });

    it('stops at the editor once it folded a selection back to a caret', () => {
      view().dispatch({ selection: { anchor: 0, head: 5 } });
      key({ key: 'Escape' });

      expect(view().state.selection.main.empty).toBe(true);
      expect(reached).toEqual([]);
    });
  });

  describe('the focus', () => {
    it('says when it leaves the editor, and not when it goes to the search panel', () => {
      view().focus();
      expect(fixture.componentInstance.hasFocus()).toBe(true);

      openSearchPanel(view());
      const field = (fixture.nativeElement as HTMLElement).querySelector<HTMLInputElement>(
        '.cm-search input[name=search]',
      )!;
      field.focus();
      expect(blurs).toBe(0);

      field.blur();
      expect(blurs).toBe(1);
    });

    it('is given up by `blur`, which is Escape’s first step', () => {
      view().focus();
      fixture.componentInstance.blur();

      expect(fixture.componentInstance.hasFocus()).toBe(false);
      expect(blurs).toBe(1);
    });
  });

  it('speaks French in its search panel', () => {
    openSearchPanel(view());

    const field = (fixture.nativeElement as HTMLElement).querySelector<HTMLInputElement>(
      '.cm-search input[name=search]',
    )!;
    expect(field.placeholder).toBe('Rechercher');
    expect(fixture.nativeElement.querySelector('.cm-search button[name=replaceAll]')?.textContent).toBe(
      'tout remplacer',
    );
  });
});
