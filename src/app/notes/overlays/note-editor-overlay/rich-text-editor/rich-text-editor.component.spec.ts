import { ComponentFixture, TestBed } from '@angular/core/testing';
import { afterEach, beforeAll, beforeEach, describe, expect, it, vi } from 'vitest';
import { loadGrammar } from '@core/services/syntax/grammars';
import type { Editor } from '@tiptap/core';
import { DiagramResult, MermaidRenderer } from '@core/services/diagrams/mermaid-renderer';
import { DrawingPicturesStore } from '@core/state/drawing-pictures.store';
import { provideTranslocoTesting } from '@testing/provide-transloco-testing';
import { RichTextEditorComponent } from './rich-text-editor.component';

describe('RichTextEditorComponent', () => {
  let fixture: ComponentFixture<RichTextEditorComponent>;
  let emitted: string[];
  /** What the editor asked to draw, and what drawing it gives. */
  let drawn: string[];
  let drawing: (source: string) => DiagramResult;
  /** The drawings a Note can embed, and their pictures by id. */
  let drawings: { id: string; title: string }[];
  let pictures: Record<string, string>;

  async function open(content: string): Promise<void> {
    fixture.componentRef.setInput('content', content);
    await fixture.whenStable();
  }

  function surface(): HTMLElement {
    return fixture.nativeElement.querySelector('[data-testid="editor-rich"]');
  }

  function button(testid: string): HTMLButtonElement {
    return fixture.nativeElement.querySelector(`[data-testid="${testid}"]`);
  }

  async function press(key: string, init: KeyboardEventInit = {}): Promise<void> {
    surface().dispatchEvent(new KeyboardEvent('keydown', { key, bubbles: true, cancelable: true, ...init }));
    await fixture.whenStable();
  }

  function paste(data: { text?: string; html?: string; types?: string[]; files?: File[] }): Event {
    const event = new Event('paste', { bubbles: true, cancelable: true });
    Object.defineProperty(event, 'clipboardData', {
      value: {
        getData: (type: string) =>
          type === 'text/plain' ? (data.text ?? '') : type === 'text/html' ? (data.html ?? '') : '',
        types: data.types ?? (data.text ? ['text/plain'] : []),
        files: data.files ?? [],
      },
    });
    surface().dispatchEvent(event);
    return event;
  }

  /** Created with its content, as a note opens: `open` replaces a document already there. */
  async function mount(content: string, linkTitle = ''): Promise<void> {
    fixture = TestBed.createComponent(RichTextEditorComponent);
    fixture.componentRef.setInput('content', content);
    fixture.componentRef.setInput('linkTitle', linkTitle);
    emitted = [];
    fixture.componentInstance.changed.subscribe((markdown) => emitted.push(markdown));
    document.body.appendChild(fixture.nativeElement);
    fixture.autoDetectChanges();
    await fixture.whenStable();
  }

  beforeEach(async () => {
    drawings = [];
    pictures = {};
    drawn = [];
    drawing = (source) => ({ svg: '<svg data-length="' + source.length + '"><text>drawn</text></svg>' });
    TestBed.configureTestingModule({
      imports: [RichTextEditorComponent],
      providers: [
        provideTranslocoTesting(),
        {
          provide: DrawingPicturesStore,
          useValue: {
            list: () => Promise.resolve(drawings),
            picture: (id: string) => Promise.resolve(pictures[id] ?? null),
          },
        },
        {
          provide: MermaidRenderer,
          useValue: {
            render: (source: string) => {
              drawn.push(source);
              return Promise.resolve(drawing(source));
            },
          },
        },
      ],
    });
    await mount('');
  });

  afterEach(() => {
    fixture.nativeElement.remove();
    vi.restoreAllMocks();
  });

  /** Headings, what marks words, blocks, then what is inserted: the mockup's order. */
  it('lays its tools out in four groups, the link among the marks, then the drawing menu', () => {
    const groups: HTMLElement[] = [...fixture.nativeElement.querySelectorAll('.rich-group')];
    const ids = groups.map((group) =>
      [...group.querySelectorAll('.rich-tool')].map((tool) => tool.getAttribute('data-testid')),
    );

    expect(ids).toEqual([
      ['rich-h1', 'rich-h2', 'rich-h3'],
      ['rich-bold', 'rich-italic', 'rich-strike', 'rich-code', 'rich-link'],
      ['rich-bullet', 'rich-ordered', 'rich-tasks', 'rich-quote'],
      ['rich-codeBlock', 'rich-table'],
      [],
    ]);
    expect(groups.at(-1)?.querySelector('[data-testid="rich-embed-drawing"]')).not.toBeNull();
  });

  it('names each block tool with the key that does the same', () => {
    expect(button('rich-h1').title).toContain('Ctrl+Alt+1');
    expect(button('rich-bullet').title).toContain('Ctrl+Maj+8');
    expect(button('rich-quote').title).toContain('Ctrl+Maj+B');
  });

  it('shows the Markdown it is given as formatted text', async () => {
    await open('# Title\n\n**bold** and a [link](https://example.com)\n\n- [x] done');

    expect(surface().querySelector('h1')?.textContent).toBe('Title');
    expect(surface().querySelector('strong')?.textContent).toBe('bold');
    expect(surface().querySelector('a')?.getAttribute('href')).toBe('https://example.com');
    expect(surface().querySelector<HTMLInputElement>('input[type="checkbox"]')?.checked).toBe(true);
  });

  /** Another note, a restored revision: replaced, and nothing written back for it. */
  it('replaces the document for a content it did not emit, without emitting', async () => {
    await open('first');
    await open('second');

    expect(surface().textContent).toBe('second');
    expect(emitted).toEqual([]);
  });

  it('bolds the selection from the toolbar and emits the Markdown', async () => {
    await open('hello');
    await press('a', { ctrlKey: true });

    button('rich-bold').click();
    await fixture.whenStable();

    expect(emitted.at(-1)).toBe('**hello**');
    expect(button('rich-bold').getAttribute('aria-pressed')).toBe('true');
  });

  /** Taking focus would move the caret out of the text before the command reaches it. */
  it('keeps the focus in the text when a tool is pressed', () => {
    const down = new MouseEvent('mousedown', { bubbles: true, cancelable: true });
    button('rich-italic').dispatchEvent(down);

    expect(down.defaultPrevented).toBe(true);
  });

  describe('the link form', () => {
    function field(testid: 'rich-link-text' | 'rich-link-address'): HTMLInputElement {
      return fixture.nativeElement.querySelector(`[data-testid="${testid}"]`);
    }

    function fill(testid: 'rich-link-text' | 'rich-link-address', value: string): void {
      field(testid).value = value;
      field(testid).dispatchEvent(new Event('input'));
    }

    async function apply(): Promise<void> {
      button('rich-link-apply').click();
      await fixture.whenStable();
    }

    it('opens from its tool, which stays pressed while the form is open', async () => {
      await open('runbook');
      await press('a', { ctrlKey: true });
      expect(button('rich-link').getAttribute('aria-pressed')).toBe('false');

      button('rich-link').click();
      await fixture.whenStable();

      expect(field('rich-link-text').value).toBe('runbook');
      expect(button('rich-link').getAttribute('aria-pressed')).toBe('true');
    });

    it('links the selection from Ctrl+K, starting from its words', async () => {
      await open('runbook');
      await press('a', { ctrlKey: true });
      await press('k', { ctrlKey: true });
      expect(field('rich-link-text').value).toBe('runbook');

      fill('rich-link-address', 'https://example.com/runbook');
      await apply();

      expect(emitted.at(-1)).toBe('[runbook](https://example.com/runbook)');
      expect(fixture.nativeElement.querySelector('[data-testid="rich-link-form"]')).toBeNull();
    });

    it('writes a link with its own words where nothing was selected', async () => {
      await press('k', { ctrlKey: true });

      fill('rich-link-text', 'the runbook');
      fill('rich-link-address', 'https://example.com');
      await apply();

      expect(emitted.at(-1)).toBe('[the runbook](https://example.com)');
    });

    it('takes the whole link under the caret, and rewords it without losing its address', async () => {
      await open('see [runbook](https://example.com) first');
      (surface() as HTMLElement & { editor: Editor }).editor.commands.setTextSelection(8);
      await press('k', { ctrlKey: true });
      expect(field('rich-link-text').value).toBe('runbook');

      fill('rich-link-text', 'the runbook');
      await apply();

      expect(emitted.at(-1)).toBe('see [the runbook](https://example.com) first');
    });

    it('shows the address when no words are given', async () => {
      await press('k', { ctrlKey: true });

      fill('rich-link-address', 'https://example.com');
      await apply();

      expect(surface().querySelector('a')?.textContent).toBe('https://example.com');
    });

    it('offers the address of the link under the caret, and an empty one takes the link off', async () => {
      await open('[runbook](https://example.com)');
      await press('a', { ctrlKey: true });
      await press('k', { ctrlKey: true });
      expect(field('rich-link-address').value).toBe('https://example.com');

      fill('rich-link-address', '');
      await apply();

      expect(emitted.at(-1)).toBe('runbook');
    });

    it('closes on Escape without letting the key reach the dialog', async () => {
      await press('k', { ctrlKey: true });
      let reached = false;
      const listener = (): void => {
        reached = true;
      };
      document.addEventListener('keydown', listener);

      field('rich-link-text').dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
      await fixture.whenStable();
      document.removeEventListener('keydown', listener);

      expect(fixture.nativeElement.querySelector('[data-testid="rich-link-form"]')).toBeNull();
      expect(reached).toBe(false);
    });
  });

  /** A plain click edits a link: the pointer shows it opens only while Ctrl is held. */
  it('points at a link while Ctrl is held, and says so on hover', async () => {
    fixture.nativeElement.remove();
    await mount('[runbook](https://example.com)', 'Ctrl+click to open');
    const live = (): boolean => surface().parentElement!.classList.contains('links-live');

    surface()
      .querySelector('a')!
      .dispatchEvent(new MouseEvent('mousemove', { bubbles: true }));
    await fixture.whenStable();
    expect(surface().parentElement!.getAttribute('title')).toBe('Ctrl+click to open');
    expect(live()).toBe(false);

    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Control', ctrlKey: true }));
    await fixture.whenStable();
    expect(live()).toBe(true);

    document.dispatchEvent(new KeyboardEvent('keyup', { key: 'Control' }));
    await fixture.whenStable();
    expect(live()).toBe(false);
  });

  /** Kept from the dialog's focus trap: a Tab it sees already answered is left alone. */
  it('types a tab rather than leaving the text, and Shift+Tab takes it back', async () => {
    await open('hello');
    (surface() as HTMLElement & { editor: Editor }).editor.commands.setTextSelection(1);
    const tab = new KeyboardEvent('keydown', { key: 'Tab', bubbles: true, cancelable: true });

    surface().dispatchEvent(tab);
    await fixture.whenStable();
    expect(tab.defaultPrevented).toBe(true);
    expect(emitted.at(-1)).toBe('&#9;hello');

    await press('Tab', { shiftKey: true });
    expect(emitted.at(-1)).toBe('hello');
  });

  describe('a code block', () => {
    // jsdom lays nothing out, and ProseMirror asks a range for its boxes to scroll to the caret.
    beforeAll(() => {
      Range.prototype.getClientRects = () => [] as unknown as DOMRectList;
      Range.prototype.getBoundingClientRect = () =>
        ({ x: 0, y: 0, width: 0, height: 0, top: 0, left: 0, right: 0, bottom: 0 }) as DOMRect;
    });

    function editor(): Editor {
      return (surface() as HTMLElement & { editor: Editor }).editor;
    }

    function languagePicker(): HTMLButtonElement | null {
      return fixture.nativeElement.querySelector('[data-testid="rich-code-language"]');
    }

    async function chooseLanguage(id: string): Promise<void> {
      languagePicker()!.click();
      await fixture.whenStable();
      fixture.nativeElement
        .querySelector(`[data-testid="choice-panel-rich-code-language"] [data-option-id="${id}"]`)
        .click();
      await fixture.whenStable();
    }

    it('is fenced, and written as typed: no escape inside it, a tab kept a tab', async () => {
      await mount('```sh\n\techo *a*_b &#9;\n```');
      editor().commands.setTextSelection(editor().state.doc.content.size - 1);
      editor().commands.insertContent(' ');
      await fixture.whenStable();

      expect(emitted.at(-1)).toBe('```sh\n\techo *a*_b &#9; \n```');
    });

    it('turns a paragraph into a block from the toolbar', async () => {
      await open('SELECT 1');
      editor().commands.setTextSelection(2);

      button('rich-codeBlock').click();
      await fixture.whenStable();

      expect(emitted.at(-1)).toBe('```\nSELECT 1\n```');
    });

    it('offers its language while the caret is in it, and writes the one chosen', async () => {
      await open('```\nSELECT 1\n```\n\nafter');
      editor().commands.setTextSelection(editor().state.doc.content.size - 1);
      await fixture.whenStable();
      expect(languagePicker()).toBeNull();

      editor().commands.setTextSelection(3);
      await fixture.whenStable();
      expect(languagePicker()?.textContent).toContain('TXT');

      await chooseLanguage('sql');

      expect(emitted.at(-1)).toBe('```sql\nSELECT 1\n```\n\nafter');
      expect(languagePicker()?.textContent).toContain('SQL');
    });

    /** The themed menu, never the operating system's `<select>`. */
    it('picks the language from a menu that keeps the caret in the block', async () => {
      await open('```\nSELECT 1\n```\n\nafter');
      editor().commands.setTextSelection(4);
      await fixture.whenStable();

      expect(fixture.nativeElement.querySelector('select')).toBeNull();
      expect(languagePicker()?.getAttribute('aria-haspopup')).toBe('listbox');
      const press = new MouseEvent('mousedown', { bubbles: true, cancelable: true });
      languagePicker()!.dispatchEvent(press);
      expect(press.defaultPrevented).toBe(true);

      await chooseLanguage('py');

      expect(editor().state.selection.from).toBe(4);
      expect(editor().state.selection.$from.parent.type.name).toBe('codeBlock');
      expect(editor().isFocused).toBe(true);
      expect(emitted.at(-1)).toBe('```py\nSELECT 1\n```\n\nafter');
    });

    it('closes its menu on Escape without the key reaching the dialog', async () => {
      await open('```\nSELECT 1\n```');
      editor().commands.setTextSelection(3);
      await fixture.whenStable();
      languagePicker()!.click();
      await fixture.whenStable();
      const reached = vi.fn();
      document.addEventListener('keydown', reached);

      languagePicker()!.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
      await fixture.whenStable();
      document.removeEventListener('keydown', reached);

      expect(
        fixture.nativeElement.querySelector('[data-testid="choice-panel-rich-code-language"]'),
      ).toBeNull();
      expect(reached).not.toHaveBeenCalled();
    });

    it('colours its code with the grammar of its language, and labels it', async () => {
      await open('```sql\nSELECT 1\n```');

      const block = surface().querySelector('pre')!;
      expect(block.getAttribute('data-language')).toBe('SQL');
      await loadGrammar('sql');
      await fixture.whenStable();
      expect(surface().querySelector('pre .tok-keyword')?.textContent).toBe('SELECT');
    });

    describe('a drawing embedded', () => {
      const embed = (): HTMLElement | null => surface().querySelector('[data-testid="rich-embed"]');

      it('is drawn from its id, the Note holding only where to ask', async () => {
        pictures['d1'] = 'data:image/png;base64,P';
        await open('Before ![Archi](devnotes:drawing/d1) after');

        await vi.waitFor(() =>
          expect(embed()?.querySelector('img')?.getAttribute('src')).toBe('data:image/png;base64,P'),
        );
        expect(embed()?.querySelector('img')?.getAttribute('alt')).toBe('Archi');
        editor().commands.insertContentAt(1, 'x');
        await fixture.whenStable();
        expect(emitted.at(-1)).toBe('xBefore ![Archi](devnotes:drawing/d1) after');
      });

      it('says when the drawing is gone, under the title it had', async () => {
        await open('![Archi](devnotes:drawing/gone)');

        await vi.waitFor(() => expect(embed()?.textContent).toBe('Dessin introuvable · Archi'));
        expect(embed()?.classList).toContain('is-missing');
      });

      it('leaves any other image as it was written, and loads nothing', async () => {
        await open('![logo](https://example.com/logo.png)');

        expect(embed()?.textContent).toBe('![logo](https://example.com/logo.png)');
        expect(embed()?.querySelector('img')).toBeNull();
      });

      it('is inserted from the toolbar, by its id under its title', async () => {
        drawings = [{ id: 'd1', title: 'Archi' }];
        await mount('Plan');
        expect(fixture.nativeElement.querySelector('[data-testid="rich-embed-drawing"]')).not.toBeNull();
        editor().commands.setTextSelection(5);

        fixture.nativeElement.querySelector('[data-testid="rich-embed-drawing"]').click();
        await fixture.whenStable();
        fixture.nativeElement
          .querySelector('[data-testid="choice-panel-rich-drawing"] [data-option-id="d1"]')
          .click();
        await fixture.whenStable();

        expect(emitted.at(-1)).toBe('Plan![Archi](devnotes:drawing/d1)');
      });

      /** DES-13: with no drawing the button was not drawn, and its menu never said why. */
      it('is offered while there is no drawing, its menu saying there is none to insert', async () => {
        fixture.nativeElement.querySelector('[data-testid="rich-embed-drawing"]').click();
        await fixture.whenStable();

        const panel = fixture.nativeElement.querySelector('[data-testid="choice-panel-rich-drawing"]');
        expect(panel.querySelector('[data-testid="choice-option"]')).toBeNull();
        expect(panel.querySelector('[data-testid="choice-empty"]')).not.toBeNull();
      });
    });

    describe('in Mermaid', () => {
      const FLOW = '~~~mermaid\ngraph TD\nA-->B\n~~~'.replaceAll('~~~', '```');
      const diagram = (): HTMLElement | null => surface().querySelector('[data-testid="rich-diagram"]');
      const block = (): HTMLElement => surface().querySelector('pre')!;

      /** jsdom does not focus an editable element: the event TipTap listens to is sent by hand. */
      async function focusAt(position: number): Promise<void> {
        editor().commands.setTextSelection(position);
        surface().dispatchEvent(new FocusEvent('focus'));
        await fixture.whenStable();
      }

      afterEach(() => {
        delete document.documentElement.dataset['theme'];
      });

      it('is drawn under its code, which folds away from the caret', async () => {
        await open(FLOW);

        await vi.waitFor(() => expect(diagram()?.querySelector('svg')).not.toBeNull());
        expect(drawn).toEqual(['graph TD\nA-->B']);
        expect(block().classList).toContain('is-folded');
        expect(block().getAttribute('data-language')).toBe('MERMAID');
        expect(diagram()!.title).toBe('Cliquer pour modifier le code du diagramme');
      });

      it('unfolds while the caret is in it, and is drawn again once the keys pause', async () => {
        await open(`${FLOW}\n\nafter`);
        await vi.waitFor(() => expect(drawn).toHaveLength(1));
        await focusAt(3);
        expect(block().classList).not.toContain('is-folded');

        editor().commands.insertContent('C');
        await fixture.whenStable();

        expect(diagram()?.querySelector('svg')).not.toBeNull();
        await vi.waitFor(() => expect(drawn).toHaveLength(2));
        expect(drawn[1]).toContain('C');
      });

      it('folds again once the editor is left', async () => {
        await open(`${FLOW}\n\nafter`);
        await focusAt(3);
        expect(block().classList).not.toContain('is-folded');

        surface().dispatchEvent(new FocusEvent('blur'));
        await fixture.whenStable();

        expect(block().classList).toContain('is-folded');
      });

      it('goes back to its code when the drawing is pressed', async () => {
        await open(`${FLOW}\n\nafter`);
        await vi.waitFor(() => expect(diagram()?.querySelector('svg')).not.toBeNull());

        diagram()!.dispatchEvent(new MouseEvent('mousedown', { bubbles: true, cancelable: true }));
        await fixture.whenStable();

        expect(editor().state.selection.$from.parent.attrs['language']).toBe('mermaid');
        expect(languagePicker()?.textContent).toContain('MERMAID');
      });

      it('says on which line a diagram cannot be read, and what Mermaid found', async () => {
        drawing = () => ({ error: 'Parse error on line 2', line: 2 });
        await open(FLOW);

        await vi.waitFor(() =>
          expect(diagram()?.querySelector('.rich-diagram-error')?.textContent).toBe(
            'Le diagramme ne se lit pas, ligne 2.',
          ),
        );
        expect(diagram()!.classList).toContain('is-failed');
        expect(diagram()!.querySelector('.rich-diagram-detail')?.textContent).toBe('Parse error on line 2');
      });

      it('says it cannot be read when Mermaid names no line', async () => {
        drawing = () => ({ error: 'No diagram type detected', line: null });
        await open(FLOW);

        await vi.waitFor(() =>
          expect(diagram()?.querySelector('.rich-diagram-error')?.textContent).toBe(
            'Le diagramme ne se lit pas.',
          ),
        );
      });

      it('is drawn again in the colours of a theme changed meanwhile', async () => {
        await open(FLOW);
        await vi.waitFor(() => expect(drawn).toHaveLength(1));

        document.documentElement.dataset['theme'] = 'light';

        await vi.waitFor(() => expect(drawn).toHaveLength(2));
      });

      it('draws nothing for an empty block, nor for one gone before its turn', async () => {
        await open(FLOW.replace('graph TD\nA-->B\n', ''));
        editor().commands.setContent(FLOW, { contentType: 'markdown' });
        editor().commands.setContent('plain', { contentType: 'markdown' });
        await new Promise((resolve) => setTimeout(resolve, 20));

        expect(drawn).toEqual([]);
        expect(diagram()).toBeNull();
      });

      it('is offered first among the languages, and written in its fence', async () => {
        const plain = FLOW.replace('mermaid', '').replace('A-->B\n', '');
        await open(`${plain}\n\nafter`);
        editor().commands.setTextSelection(3);
        await fixture.whenStable();

        languagePicker()!.click();
        await fixture.whenStable();
        const first = fixture.nativeElement.querySelector(
          '[data-testid="choice-panel-rich-code-language"] [data-option-id]',
        );
        expect(first?.getAttribute('data-option-id')).toBe('mermaid');
        first.click();
        await fixture.whenStable();

        expect(emitted.at(-1)).toBe(`${FLOW.replace('A-->B\n', '')}\n\nafter`);
      });
    });

    /** The code field's level, not the prose's tab. */
    it('indents with Tab as the code field would for its language', async () => {
      fixture.componentRef.setInput('codeIndent', 'language');
      await open('```py\npass\n```');
      editor().commands.setTextSelection(1);

      await press('Tab');

      expect(emitted.at(-1)).toBe('```py\n    pass\n```');
    });
  });

  it('offers the table tools while the caret is in a table, and only then', async () => {
    expect(button('rich-rowAfter')).toBeNull();

    button('rich-table').click();
    await fixture.whenStable();

    expect(button('rich-rowAfter')).not.toBeNull();
    expect(emitted.at(-1)).toContain('| --- |');
  });

  it.each([
    ['h1', '# hello'],
    ['h2', '## hello'],
    ['h3', '### hello'],
    ['italic', '*hello*'],
    ['strike', '~~hello~~'],
    ['code', '`hello`'],
    ['bullet', '- hello'],
    ['ordered', '1. hello'],
    ['tasks', '- [ ] hello'],
    ['quote', '> hello'],
  ])('writes what %s does as Markdown', async (tool, markdown) => {
    await open('hello');
    await press('a', { ctrlKey: true });

    button(`rich-${tool}`).click();
    await fixture.whenStable();

    expect(emitted.at(-1)).toBe(markdown);
  });

  it('grows and shrinks a table from its tools, then takes it away', async () => {
    const rows = (): number => surface().querySelectorAll('tr').length;
    const columns = (): number => surface().querySelector('tr')?.children.length ?? 0;
    const use = async (tool: string): Promise<void> => {
      button(`rich-${tool}`).click();
      await fixture.whenStable();
    };

    await use('table');
    expect([rows(), columns()]).toEqual([3, 3]);

    await use('rowAfter');
    await use('columnAfter');
    expect([rows(), columns()]).toEqual([4, 4]);

    await use('deleteRow');
    await use('deleteColumn');
    expect([rows(), columns()]).toEqual([3, 3]);

    await use('deleteTable');
    expect(surface().querySelector('table')).toBeNull();
    expect(button('rich-rowAfter')).toBeNull();
  });

  it('says whether the text holds the focus, and when it lets it go', () => {
    let blurs = 0;
    fixture.componentInstance.blurred.subscribe(() => (blurs += 1));
    expect(fixture.componentInstance.hasFocus()).toBe(false);

    surface().focus();
    expect(fixture.componentInstance.hasFocus()).toBe(true);

    surface().blur();
    expect(fixture.componentInstance.hasFocus()).toBe(false);
    expect(blurs).toBe(1);
  });

  /** RIC-09: typed key by key, « 2 * 3 * 4 » lost its stars to an italic 3. */
  describe('emphasis typed key by key', () => {
    /** As ProseMirror types: the input rules see each character before it lands. */
    async function typeIn(text: string): Promise<void> {
      const view = (surface() as HTMLElement & { editor: Editor }).editor.view;
      for (const char of text) {
        const { from, to } = view.state.selection;
        const handled = view.someProp('handleTextInput', (handle) =>
          handle(view, from, to, char, () => view.state.tr.insertText(char, from, to)),
        );
        if (!handled) view.dispatch(view.state.tr.insertText(char, from, to));
      }
      await fixture.whenStable();
    }

    it('keeps a star or an underscore with a space inside as it is', async () => {
      await open('');
      await typeIn('2 * 3 * 4 and a _ b _ c');

      expect(surface().querySelector('em')).toBeNull();
      expect(emitted.at(-1)).toBe('2 * 3 * 4 and a _ b _ c');
    });

    it('still makes italic what Markdown makes italic', async () => {
      await open('');
      await typeIn('a *b c* d');

      expect(surface().querySelector('em')?.textContent).toBe('b c');
      expect(emitted.at(-1)).toBe('a *b c* d');
    });
  });

  /** jsdom has no layout for ProseMirror to find a click in: its click hook is asked directly. */
  it('opens a link on Ctrl+click, and leaves a plain click to the caret', async () => {
    await open('[runbook](https://example.com)');
    const opened: string[] = [];
    fixture.componentInstance.linkOpened.subscribe((href) => opened.push(href));
    const view = (surface() as HTMLElement & { editor: Editor }).editor.view;
    const click = (init: MouseEventInit): boolean => {
      const event = new MouseEvent('click', { cancelable: true, ...init });
      Object.defineProperty(event, 'target', { value: surface().querySelector('a') });
      return view.someProp('handleClick', (handle) => handle(view, 1, event)) ?? false;
    };

    expect(click({})).toBe(false);
    expect(click({ ctrlKey: true })).toBe(true);
    expect(opened).toEqual(['https://example.com']);
  });

  /** RIC-06: the hook left a plain click alone, and the browser's click followed the link. */
  it('never lets the click itself follow a link, plain or with Ctrl', async () => {
    await open('[runbook](https://example.com)');
    const anchor = surface().querySelector('a')!;
    const clicked = (init: MouseEventInit): boolean => {
      const event = new MouseEvent('click', { bubbles: true, cancelable: true, ...init });
      anchor.dispatchEvent(event);
      return event.defaultPrevented;
    };

    expect(clicked({})).toBe(true);
    expect(clicked({ ctrlKey: true })).toBe(true);
    const text = document.createElement('span');
    surface().append(text);
    const plain = new MouseEvent('click', { bubbles: true, cancelable: true });
    text.dispatchEvent(plain);
    expect(plain.defaultPrevented).toBe(false);
  });

  /**
   * WebKit, which Linux runs, focuses inside TipTap's `chain().focus()`, and that focus dispatches
   * first: a note ending in a list gains its trailing paragraph under the chain's transaction.
   */
  describe('on WebKit, with the text not focused yet', () => {
    beforeEach(() => {
      vi.spyOn(navigator, 'userAgent', 'get').mockReturnValue(
        'Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.0 Safari/605.1.15',
      );
      // jsdom lays nothing out, and a focused view scrolls to its caret.
      Object.assign(Range.prototype, {
        getClientRects: () => [],
        getBoundingClientRect: () => ({ top: 0, bottom: 0, left: 0, right: 0, width: 0, height: 0 }),
      });
    });

    afterEach(() => {
      Reflect.deleteProperty(Range.prototype, 'getClientRects');
      Reflect.deleteProperty(Range.prototype, 'getBoundingClientRect');
    });

    it('ticks a box', async () => {
      fixture.nativeElement.remove();
      await mount('- [ ] ship it');
      const box = surface().querySelector<HTMLInputElement>('input[type="checkbox"]')!;

      box.click();
      await fixture.whenStable();

      expect(emitted.at(-1)).toBe('- [x] ship it');
    });

    it('runs a tool', async () => {
      fixture.nativeElement.remove();
      await mount('- [ ] ship it');

      button('rich-tasks').click();
      await fixture.whenStable();

      expect(emitted.at(-1)).toBe('ship it');
    });
  });

  describe('a paste', () => {
    it('hands plain text over when the note is empty, and pastes nothing itself', async () => {
      const pasted: string[] = [];
      fixture.componentInstance.pastedIntoEmpty.subscribe((text) => pasted.push(text));

      const event = paste({ text: 'SELECT 1;' });
      await fixture.whenStable();

      expect(pasted).toEqual(['SELECT 1;']);
      expect(event.defaultPrevented).toBe(true);
      expect(emitted).toEqual([]);
    });

    /** RIC-10: pasted into a new, empty Note, a page's bold and link arrived as plain text. */
    it('pastes the bold and the link of a page into an empty note as they are', async () => {
      const pasted: string[] = [];
      fixture.componentInstance.pastedIntoEmpty.subscribe((text) => pasted.push(text));

      paste({
        text: 'Voir ceci et là',
        html: '<p>Voir <strong>ceci</strong> et <a href="https://devnotes.fr">là</a></p>',
      });
      await fixture.whenStable();

      expect(pasted).toEqual([]);
      expect(surface().querySelector('strong')?.textContent).toBe('ceci');
      expect(surface().querySelector('a')?.getAttribute('href')).toBe('https://devnotes.fr');
    });

    it('still asks about code an editor copied as coloured HTML', async () => {
      const pasted: string[] = [];
      fixture.componentInstance.pastedIntoEmpty.subscribe((text) => pasted.push(text));

      paste({
        text: 'const a = 1;',
        html: '<div style="color:#d4d4d4"><span style="color:#569cd6">const</span> a = 1;</div>',
      });
      paste({ text: 'npm run build', html: '<pre><code><b>npm</b> run build</code></pre>' });

      expect(pasted).toEqual(['const a = 1;', 'npm run build']);
    });

    it('leaves a paste into a note that has text to the editor', async () => {
      await open('already here');
      const pasted: string[] = [];
      fixture.componentInstance.pastedIntoEmpty.subscribe((text) => pasted.push(text));

      paste({ text: 'more' });
      await fixture.whenStable();

      expect(pasted).toEqual([]);
    });

    it('turns an image with no text into an attachment request', async () => {
      let images = 0;
      fixture.componentInstance.imagePasted.subscribe(() => (images += 1));

      paste({ types: ['image/png'] });
      await fixture.whenStable();

      expect(images).toBe(1);
    });
  });

  describe('searched', () => {
    const bar = (): HTMLElement | null => fixture.nativeElement.querySelector('[data-testid="rich-search"]');
    const findField = (): HTMLInputElement =>
      fixture.nativeElement.querySelector('[data-testid="search-field"]');

    /** The code field's bar, over the text: Ctrl+F on what is selected, Escape back to the text. */
    it('opens on Ctrl+F with the selected words, counts as it is typed, and closes on Escape', async () => {
      await mount('deploy the build, then deploy again');
      const editor = (fixture.componentInstance as unknown as { editor: Editor }).editor;
      editor.commands.setTextSelection({ from: 1, to: 7 });

      await press('f', { ctrlKey: true });
      await vi.waitFor(() => expect(document.activeElement).toBe(findField()));
      expect(findField().value).toBe('deploy');
      expect(fixture.nativeElement.querySelector('[data-testid="search-count"]').textContent.trim()).toBe(
        '1/2',
      );

      const escape = new KeyboardEvent('keydown', { key: 'Escape', bubbles: true, cancelable: true });
      const dialog = vi.fn();
      document.addEventListener('keydown', dialog);
      findField().dispatchEvent(escape);
      await fixture.whenStable();
      document.removeEventListener('keydown', dialog);

      expect(bar()).toBeNull();
      expect(dialog).not.toHaveBeenCalled();
      expect(fixture.nativeElement.querySelector('.search-match')).toBeNull();
    });

    it('opens with its replace row on Ctrl+H, and writes what it replaced', async () => {
      await mount('deploy the build, then deploy again');

      await press('h', { ctrlKey: true });
      const replaceField = (): HTMLInputElement | null =>
        fixture.nativeElement.querySelector('[data-testid="search-replace-field"]');
      await vi.waitFor(() => expect(replaceField()).not.toBeNull());
      for (const [input, value] of [
        [findField(), 'deploy'],
        [replaceField()!, 'ship'],
      ] as const) {
        input.value = value;
        input.dispatchEvent(new Event('input'));
      }
      button('search-replace-all').click();
      await fixture.whenStable();

      expect(emitted.at(-1)).toBe('ship the build, then ship again');
    });
  });
});
