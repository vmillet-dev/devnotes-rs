import { ComponentFixture, TestBed } from '@angular/core/testing';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { undo } from '@codemirror/commands';
import { openSearchPanel } from '@codemirror/search';
import { EditorView } from '@codemirror/view';
import { PreferencesService } from '@core/services/preferences/preferences.service';
import { provideTranslocoTesting } from '@testing/provide-transloco-testing';
import { CodeEditorComponent } from '../code-editor.component';

/** The bar as CodeMirror opens it: over a real editor, as in the overlay. */
describe('SearchBarComponent', () => {
  let fixture: ComponentFixture<CodeEditorComponent>;

  const root = (): HTMLElement => fixture.nativeElement as HTMLElement;
  const part = <T extends HTMLElement = HTMLElement>(id: string): T | null =>
    root().querySelector<T>(`[data-testid="${id}"]`);
  const field = (): HTMLInputElement => part<HTMLInputElement>('search-field')!;
  const count = (): string => (part('search-count')?.textContent ?? '').replace(/\s/g, '');
  const status = (): string => (root().querySelector('[role="status"]')?.textContent ?? '').trim();

  function view(): EditorView {
    return EditorView.findFromDOM(root().querySelector('.cm-editor')!)!;
  }

  function doc(): string {
    return view().state.doc.toString();
  }

  function selected(): string {
    const { from, to } = view().state.selection.main;
    return `${from}-${to}`;
  }

  async function mount(content: string, session = 1): Promise<void> {
    fixture = TestBed.createComponent(CodeEditorComponent);
    const inputs = { session, content, language: 'txt', indentUnit: '  ' };
    for (const [name, value] of Object.entries(inputs)) fixture.componentRef.setInput(name, value);
    document.body.appendChild(fixture.nativeElement);
    fixture.autoDetectChanges();
    await fixture.whenStable();
  }

  async function open(caret = 0): Promise<void> {
    view().dispatch({ selection: { anchor: caret } });
    openSearchPanel(view());
    await fixture.whenStable();
  }

  async function type(input: HTMLInputElement, text: string): Promise<void> {
    input.value = text;
    input.dispatchEvent(new Event('input'));
    await fixture.whenStable();
  }

  async function press(target: HTMLElement, init: KeyboardEventInit): Promise<KeyboardEvent> {
    const event = new KeyboardEvent('keydown', { bubbles: true, cancelable: true, ...init });
    target.dispatchEvent(event);
    await fixture.whenStable();
    return event;
  }

  async function click(id: string): Promise<void> {
    part(id)!.click();
    await fixture.whenStable();
  }

  beforeEach(async () => {
    TestBed.configureTestingModule({
      imports: [CodeEditorComponent],
      providers: [provideTranslocoTesting()],
    });
    await mount('alpha beta\nbeta gamma\nalpha beta');
  });

  afterEach(() => fixture.nativeElement.remove());

  it('opens on the selected text, the field focused', async () => {
    view().dispatch({ selection: { anchor: 11, head: 15 } });
    openSearchPanel(view());
    await fixture.whenStable();

    expect(field().value).toBe('beta');
    expect(document.activeElement).toBe(field());
    expect(count()).toBe('2/3');
  });

  it('selects the first match from where it started as it is typed, and counts them', async () => {
    await open(11);

    await type(field(), 'beta');

    expect(selected()).toBe('11-15');
    expect(count()).toBe('2/3');
    expect(status()).toBe('3 résultats');
    expect(root().querySelectorAll('.cm-searchMatch')).toHaveLength(3);
  });

  it('wraps round to the start for a match before where it started', async () => {
    await open(20);

    await type(field(), 'alpha');

    expect(selected()).toBe('22-27');
    await type(field(), 'gamma');
    expect(selected()).toBe('16-21');
  });

  it('says 0, in red, when nothing matches', async () => {
    await open();

    await type(field(), 'zeta');

    expect(count()).toBe('0');
    expect(part('search-count')!.classList.contains('none')).toBe(true);
    expect(root().querySelector('.field')!.classList.contains('none')).toBe(true);
    expect(status()).toBe('Aucun résultat');
  });

  it('counts the matches alone while the selection is on none of them', async () => {
    await open();
    await type(field(), 'beta');

    view().dispatch({ selection: { anchor: 0 } });
    await fixture.whenStable();

    expect(count()).toBe('3');
  });

  it('moves between matches with Enter, Shift+Enter and the arrows', async () => {
    await open();
    await type(field(), 'beta');
    expect(count()).toBe('1/3');

    await press(field(), { key: 'Enter' });
    expect(count()).toBe('2/3');
    await press(field(), { key: 'Enter', shiftKey: true });
    expect(count()).toBe('1/3');
    await click('search-next');
    expect(count()).toBe('2/3');
    await click('search-previous');
    expect(count()).toBe('1/3');
  });

  it('matches case, whole words and expressions, each a pressed button', async () => {
    fixture.nativeElement.remove();
    await mount('Beta beta betas b.ta');
    await open();
    await type(field(), 'beta');
    expect(count()).toBe('1/3');

    await click('search-case');
    expect(part('search-case')!.getAttribute('aria-pressed')).toBe('true');
    expect(count().split('/')[1]).toBe('2');

    await click('search-word');
    expect(part('search-word')!.getAttribute('aria-pressed')).toBe('true');
    expect(count().split('/')[1]).toBe('1');

    await type(field(), 'b.ta');
    expect(count().split('/')[1]).toBe('1');
    await click('search-regexp');
    expect(part('search-regexp')!.getAttribute('aria-pressed')).toBe('true');
    expect(count().split('/')[1]).toBe('2');
  });

  it('remembers the options for the next note', async () => {
    await open();
    await click('search-case');
    expect(JSON.parse(TestBed.inject(PreferencesService).read('devnotes.editorSearchOptions')!)).toEqual({
      caseSensitive: true,
      wholeWord: false,
      regexp: false,
    });

    fixture.componentRef.setInput('session', 2);
    fixture.componentRef.setInput('content', 'another note');
    await fixture.whenStable();
    await open();

    expect(part('search-case')!.getAttribute('aria-pressed')).toBe('true');
    expect(part('search-word')!.getAttribute('aria-pressed')).toBe('false');
  });

  it('keeps replacing folded until the chevron or Ctrl+H asks for it', async () => {
    await open();
    expect(part('search-replace-field')).toBeNull();
    expect(part('search-replace-toggle')!.getAttribute('aria-expanded')).toBe('false');

    await click('search-replace-toggle');
    expect(part('search-replace-field')).not.toBeNull();
    expect(document.activeElement).toBe(part('search-replace-field'));

    await click('search-replace-toggle');
    expect(part('search-replace-field')).toBeNull();

    await press(field(), { key: 'h', ctrlKey: true });
    expect(part('search-replace-field')).not.toBeNull();
  });

  it('opens on its replace row with Ctrl+H from the code', async () => {
    view().focus();

    await press(view().contentDOM, { key: 'h', ctrlKey: true });

    expect(part('search-replace-field')).not.toBeNull();
  });

  it('replaces one match, then all of them, each undone in one step', async () => {
    await open();
    await type(field(), 'beta');
    await click('search-replace-toggle');
    await type(part<HTMLInputElement>('search-replace-field')!, 'BETA');

    await click('search-replace-one');
    expect(doc()).toBe('alpha BETA\nbeta gamma\nalpha beta');

    await press(part('search-replace-field')!, { key: 'Enter' });
    expect(doc()).toBe('alpha BETA\nBETA gamma\nalpha beta');

    await click('search-replace-all');
    expect(doc()).toBe('alpha BETA\nBETA gamma\nalpha BETA');

    undo(view());
    expect(doc()).toBe('alpha BETA\nBETA gamma\nalpha beta');
    undo(view());
    expect(doc()).toBe('alpha BETA\nbeta gamma\nalpha beta');
  });

  it('closes on Escape, the caret back in the code and the dialog left alone', async () => {
    const reached: Event[] = [];
    const listen = (event: Event) => reached.push(event);
    document.addEventListener('keydown', listen);
    await open();

    const event = await press(field(), { key: 'Escape' });

    expect(event.defaultPrevented).toBe(true);
    expect(reached).toEqual([]);
    expect(field()).toBeNull();
    expect(view().hasFocus).toBe(true);
    document.removeEventListener('keydown', listen);
  });

  it('closes from its button', async () => {
    await open();

    await click('search-close');

    expect(part('search-field')).toBeNull();
  });

  it('stops counting at a thousand', async () => {
    fixture.nativeElement.remove();
    await mount('ab '.repeat(1500));
    await open();

    await type(field(), 'ab');

    expect(count()).toBe('1/1000+');
  });

  it('counts a large text once the typing pauses', async () => {
    fixture.nativeElement.remove();
    await mount(`${'x'.repeat(250_000)} needle`);
    await open();

    await type(field(), 'needle');
    expect(count()).toBe('…');
    expect(part('search-count')!.classList.contains('none')).toBe(false);

    await new Promise((resolve) => setTimeout(resolve, 400));
    await fixture.whenStable();
    expect(count()).toBe('1/1');
  });
});
