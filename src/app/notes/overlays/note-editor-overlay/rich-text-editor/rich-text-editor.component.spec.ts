import { ComponentFixture, TestBed } from '@angular/core/testing';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
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

  beforeEach(async () => {
    TestBed.configureTestingModule({
      imports: [RichTextEditorComponent],
      providers: [provideTranslocoTesting()],
    });
    fixture = TestBed.createComponent(RichTextEditorComponent);
    fixture.componentRef.setInput('content', '');
    emitted = [];
    fixture.componentInstance.changed.subscribe((markdown) => emitted.push(markdown));
    document.body.appendChild(fixture.nativeElement);
    fixture.autoDetectChanges();
    await open('');
  });

  afterEach(() => {
    fixture.nativeElement.remove();
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

  it('links the selection from Ctrl+K', async () => {
    await open('runbook');
    await press('a', { ctrlKey: true });
    await press('k', { ctrlKey: true });

    const field = fixture.nativeElement.querySelector('[data-testid="rich-link-field"]') as HTMLInputElement;
    field.value = 'https://example.com/runbook';
    field.dispatchEvent(new Event('input'));
    button('rich-link-apply').click();
    await fixture.whenStable();

    expect(emitted.at(-1)).toBe('[runbook](https://example.com/runbook)');
    expect(fixture.nativeElement.querySelector('[data-testid="rich-link-form"]')).toBeNull();
  });

  it('closes the link form on Escape without letting the key reach the dialog', async () => {
    await press('k', { ctrlKey: true });
    const field = fixture.nativeElement.querySelector('[data-testid="rich-link-field"]') as HTMLInputElement;
    let reached = false;
    const listener = (): void => {
      reached = true;
    };
    document.addEventListener('keydown', listener);

    field.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
    await fixture.whenStable();
    document.removeEventListener('keydown', listener);

    expect(fixture.nativeElement.querySelector('[data-testid="rich-link-form"]')).toBeNull();
    expect(reached).toBe(false);
  });

  it('offers the table tools while the caret is in a table, and only then', async () => {
    expect(button('rich-rowAfter')).toBeNull();

    button('rich-table').click();
    await fixture.whenStable();

    expect(button('rich-rowAfter')).not.toBeNull();
    expect(emitted.at(-1)).toContain('| --- |');
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
