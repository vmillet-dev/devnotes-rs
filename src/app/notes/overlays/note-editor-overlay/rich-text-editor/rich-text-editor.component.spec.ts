import { ComponentFixture, TestBed } from '@angular/core/testing';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import type { Editor } from '@tiptap/core';
import { provideTranslocoTesting } from '@testing/provide-transloco-testing';
import { RichTextEditorComponent } from './rich-text-editor.component';

describe('RichTextEditorComponent', () => {
  let fixture: ComponentFixture<RichTextEditorComponent>;
  let emitted: string[];

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

  function paste(data: { text?: string; types?: string[]; files?: File[] }): Event {
    const event = new Event('paste', { bubbles: true, cancelable: true });
    Object.defineProperty(event, 'clipboardData', {
      value: {
        getData: (type: string) => (type === 'text/plain' ? (data.text ?? '') : ''),
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
    TestBed.configureTestingModule({
      imports: [RichTextEditorComponent],
      providers: [provideTranslocoTesting()],
    });
    await mount('');
  });

  afterEach(() => {
    fixture.nativeElement.remove();
    vi.restoreAllMocks();
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
});
