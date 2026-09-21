import { ComponentFixture, TestBed } from '@angular/core/testing';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { provideTranslocoTesting } from '@testing/provide-transloco-testing';
import { SearchBoxComponent } from './search-box.component';

describe('SearchBoxComponent', () => {
  let fixture: ComponentFixture<SearchBoxComponent>;

  function input(): HTMLInputElement {
    return fixture.nativeElement.querySelector('input');
  }

  function pressShortcut(): KeyboardEvent {
    const event = new KeyboardEvent('keydown', { key: 'k', ctrlKey: true, cancelable: true });
    document.dispatchEvent(event);
    return event;
  }

  beforeEach(() => {
    TestBed.configureTestingModule({ imports: [SearchBoxComponent], providers: [provideTranslocoTesting()] });
    fixture = TestBed.createComponent(SearchBoxComponent);
    // jsdom only tracks `document.activeElement` for attached elements.
    document.body.appendChild(fixture.nativeElement);
    fixture.autoDetectChanges();
  });

  afterEach(() => {
    fixture.nativeElement.remove();
  });

  /**
   * ⚠️ The row wraps before the window is narrow, so the field spends most of its life at
   * its floor — the long form was cut mid-word and said less than nothing.
   */
  it('keeps the long form as a tooltip and a short one in the field', () => {
    const box: HTMLElement = fixture.nativeElement.querySelector('.search-bar');

    expect(box.getAttribute('title')).toBe('Rechercher une note, un tag, du code');
    expect(input().placeholder).toBe('Rechercher…');
  });

  /**
   * ⚠️ The row wraps before the window is narrow, so the field spends most of its life at
   * its floor — the long form was cut mid-word and said less than nothing.
   */
  it('keeps the long form as a tooltip and a short one in the field', () => {
    const box: HTMLElement = fixture.nativeElement.querySelector('.search-bar');

    expect(box.getAttribute('title')).toBe('Rechercher une note, un tag, du code');
    expect(input().placeholder).toBe('Rechercher…');
  });

  it('renders the provided query in the input', async () => {
    fixture.componentRef.setInput('query', 'hello');
    await fixture.whenStable();

    expect(input().value).toBe('hello');
  });

  it('emits the new value when the user types', () => {
    let emitted: string | undefined;
    fixture.componentInstance.query.subscribe((value: string) => (emitted = value));

    input().value = 'search term';
    input().dispatchEvent(new Event('input'));

    expect(emitted).toBe('search term');
  });

  it('gives the input an accessible name of its own', () => {
    expect(input().getAttribute('aria-label')).toBe('Rechercher dans les notes');
  });

  it('hides the decorative magnifier and shortcut hint from assistive tech', () => {
    const decorations = [...fixture.nativeElement.querySelectorAll('label > span')];

    expect(decorations.every((span: Element) => span.getAttribute('aria-hidden') === 'true')).toBe(true);
  });

  describe('the count', () => {
    function matchedText(): string {
      const node = fixture.nativeElement.querySelector('[data-testid="search-matched"]');
      return node.textContent.replace(/\s+/g, ' ').trim();
    }

    it('replaces the shortcut hint while something is being filtered', async () => {
      fixture.componentRef.setInput('matched', 12);
      await fixture.whenStable();

      expect(matchedText()).toBe('12 résultats');
      expect(fixture.nativeElement.querySelector('.kbd')).toBeNull();
    });

    it('says zero rather than falling back to the hint', async () => {
      // The answer that matters most, and the one a truthiness check would swallow.
      fixture.componentRef.setInput('matched', 0);
      await fixture.whenStable();

      expect(matchedText()).toBe('aucun résultat');
    });

    it('shows the hint again when nothing is being filtered', async () => {
      fixture.componentRef.setInput('matched', null);
      await fixture.whenStable();

      expect(fixture.nativeElement.querySelector('[data-testid="search-matched"]')).toBeNull();
      expect(fixture.nativeElement.querySelector('.kbd')).not.toBeNull();
    });

    /**
     * ⚠️ It used to be a button whose click dropped the tags and the languages with the
     * text, which is not what a count says and not what a cross in a field means.
     */
    it('is a count and not a control', async () => {
      fixture.componentRef.setInput('matched', 12);
      await fixture.whenStable();

      expect(fixture.nativeElement.querySelector('[data-testid="search-matched"]').tagName).toBe('SPAN');
    });
  });

  describe('the cross inside the field', () => {
    const cross = () => fixture.nativeElement.querySelector('[data-testid="search-clear"]');

    it('is absent while there is nothing to clear', () => {
      expect(cross()).toBeNull();
    });

    it('appears once something has been typed', async () => {
      fixture.componentRef.setInput('query', 'docker');
      await fixture.whenStable();

      expect(cross()).not.toBeNull();
    });

    /**
     * ⚠️ `preventDefault` matters: the field is inside the `<label>`, so the click would
     * otherwise focus it and hand the user a caret in a box they had just emptied.
     */
    it('asks for the text to go, without focusing the field it emptied', async () => {
      fixture.componentRef.setInput('query', 'docker');
      await fixture.whenStable();
      let asked = 0;
      fixture.componentInstance.textCleared.subscribe(() => (asked += 1));

      const event = new MouseEvent('click', { bubbles: true, cancelable: true });
      cross().dispatchEvent(event);
      await fixture.whenStable();

      expect(asked).toBe(1);
      expect(event.defaultPrevented).toBe(true);
    });

    it('names what it clears, the glyph alone saying nothing', async () => {
      fixture.componentRef.setInput('query', 'docker');
      await fixture.whenStable();

      expect(cross().getAttribute('aria-label')).toBe('Effacer la recherche');
    });
  });

  it('focuses the input on Ctrl/Cmd+K and prevents the browser default', () => {
    const event = pressShortcut();

    expect(document.activeElement).toBe(input());
    expect(event.defaultPrevented).toBe(true);
  });

  it('ignores keydown events that are not the shortcut', () => {
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'k' }));

    expect(document.activeElement).not.toBe(input());
  });

  it('ignores the shortcut while it is disabled', async () => {
    fixture.componentRef.setInput('shortcutEnabled', false);
    await fixture.whenStable();

    const event = pressShortcut();

    expect(document.activeElement).not.toBe(input());
    expect(event.defaultPrevented).toBe(false);
  });
});
