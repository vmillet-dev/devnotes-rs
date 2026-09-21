import { ComponentFixture, TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';
import { createNote } from '@testing/note.fixture';
import { provideTranslocoTesting } from '@testing/provide-transloco-testing';
import { QuickPaletteComponent } from './quick-palette.component';

const RESULTS = [
  createNote({ id: 'note-1', title: 'First', content: 'one\ntwo\nthree' }),
  createNote({
    id: 'note-2',
    title: 'Templated',
    content: 'psql -h {{host}}',
    placeholders: [{ name: 'host', defaultValue: '', value: '' }],
  }),
];

describe('QuickPaletteComponent', () => {
  let fixture: ComponentFixture<QuickPaletteComponent>;

  function input(): HTMLInputElement {
    return fixture.nativeElement.querySelector('.palette-input');
  }

  function options(): HTMLElement[] {
    return [...fixture.nativeElement.querySelectorAll('.palette-option')];
  }

  async function press(key: string, ctrlKey = false, shiftKey = false): Promise<void> {
    input().dispatchEvent(new KeyboardEvent('keydown', { key, ctrlKey, shiftKey, bubbles: true }));
    await fixture.whenStable();
  }

  beforeEach(async () => {
    TestBed.resetTestingModule();
    TestBed.configureTestingModule({
      imports: [QuickPaletteComponent],
      providers: [provideTranslocoTesting()],
    });
    fixture = TestBed.createComponent(QuickPaletteComponent);
    fixture.componentRef.setInput('results', RESULTS);
    fixture.autoDetectChanges();
    await fixture.whenStable();
  });

  it('takes the focus on the search field', () => {
    expect(document.activeElement).toBe(input());
  });

  it('marks the highlighted option without moving the focus', async () => {
    fixture.componentRef.setInput('highlighted', 1);
    await fixture.whenStable();

    expect(options()[1].getAttribute('aria-selected')).toBe('true');
    expect(input().getAttribute('aria-activedescendant')).toBe(options()[1].id);
    expect(document.activeElement).toBe(input());
  });

  it('walks the list with the arrow keys', async () => {
    const moves: number[] = [];
    fixture.componentInstance.highlightMoved.subscribe((step) => moves.push(step));

    await press('ArrowDown');
    await press('ArrowUp');

    expect(moves).toEqual([1, -1]);
  });

  /**
   * ⚠️ Enter used to copy and put the window away, where a click on the same row opened
   * the note and Enter opens everywhere else in the application.
   */
  it('opens the highlighted note on Enter, as a click on it does', async () => {
    let opened: string | undefined;
    let chosen = 0;
    fixture.componentInstance.openRequested.subscribe((id) => (opened = id));
    fixture.componentInstance.chosen.subscribe(() => (chosen += 1));
    fixture.componentRef.setInput('highlighted', 1);
    await fixture.whenStable();

    await press('Enter');

    expect(opened).toBe('note-2');
    expect(chosen).toBe(0);
  });

  it('copies and closes on Ctrl+C, which is what the canvas copies with', async () => {
    let chosen = 0;
    let opened = 0;
    fixture.componentInstance.chosen.subscribe(() => (chosen += 1));
    fixture.componentInstance.openRequested.subscribe(() => (opened += 1));

    await press('c', true);

    expect(chosen).toBe(1);
    expect(opened).toBe(0);
  });

  /** ⚠️ The field is a search field: a selection in it is what Ctrl+C means there. */
  it('leaves Ctrl+C to the field while text is selected in it', async () => {
    let chosen = 0;
    fixture.componentInstance.chosen.subscribe(() => (chosen += 1));
    input().value = 'psql';
    input().setSelectionRange(0, 4);

    await press('c', true);

    expect(chosen).toBe(0);
  });

  /**
   * ⚠️ Released to the DOM, Tab walked stops nothing drew a ring on while the highlight
   * stayed where the arrows had left it. One current row, and it is the highlighted one.
   */
  it('walks the list with Tab too, and opens nothing on the way', async () => {
    const moves: number[] = [];
    let opened = 0;
    fixture.componentInstance.highlightMoved.subscribe((step) => moves.push(step));
    fixture.componentInstance.openRequested.subscribe(() => (opened += 1));

    await press('Tab');
    await press('Tab', false, true);

    expect(moves).toEqual([1, -1]);
    expect(opened).toBe(0);
    expect(document.activeElement).toBe(input());
  });

  /** The other half: a stop Tab cannot reach is a stop that cannot steal the typing. */
  it('keeps every row out of the tab order', () => {
    const buttons = [...fixture.nativeElement.querySelectorAll('.palette-option button')];

    expect(buttons.length).toBeGreaterThan(0);
    expect(buttons.map((button: HTMLElement) => button.tabIndex)).toEqual(buttons.map(() => -1));
  });

  it('closes on Escape', async () => {
    let closed = 0;
    fixture.componentInstance.closed.subscribe(() => (closed += 1));

    await press('Escape');

    expect(closed).toBe(1);
  });

  it('flags a snippet that will ask for values', () => {
    expect(options()[0].querySelector('.palette-option-fields')).toBeNull();
    expect(options()[1].querySelector('.palette-option-fields')?.textContent).toContain('1');
  });

  it('trims the preview to a couple of lines', () => {
    expect(options()[0].querySelector('.palette-option-snippet')?.textContent).toBe('one\ntwo');
  });

  it('says when nothing matches', async () => {
    fixture.componentRef.setInput('results', []);
    await fixture.whenStable();

    expect(fixture.nativeElement.querySelector('.palette-empty').textContent).toContain('Aucun');
  });

  /**
   * ⚠️ A click **opens**: it is what a click on a note means everywhere else, and the
   * window is already in front. It used to copy and put the window away, which read as a
   * crash — nothing on screen said anything had been copied.
   */
  it('opens the note that was clicked, without copying it', async () => {
    let index: number | undefined;
    let opened: string | undefined;
    let chosen = 0;
    fixture.componentInstance.highlightSet.subscribe((value) => (index = value));
    fixture.componentInstance.openRequested.subscribe((id) => (opened = id));
    fixture.componentInstance.chosen.subscribe(() => (chosen += 1));

    (options()[1].querySelector('.palette-option-button') as HTMLButtonElement).click();
    await fixture.whenStable();

    expect(index).toBe(1);
    expect(opened).toBe(RESULTS[1].id);
    expect(chosen).toBe(0);
  });

  /** The paste path keeps a mouse of its own, beside the row rather than over it. */
  it('copies from the control beside the row', async () => {
    let chosen = 0;
    let opened = 0;
    fixture.componentInstance.chosen.subscribe(() => (chosen += 1));
    fixture.componentInstance.openRequested.subscribe(() => (opened += 1));

    (options()[1].querySelector('.palette-option-copy') as HTMLButtonElement).click();
    await fixture.whenStable();

    expect(chosen).toBe(1);
    expect(opened).toBe(0);
  });
  describe('the create row', () => {
    async function offerCreation(query: string): Promise<void> {
      fixture.componentRef.setInput('query', query);
      fixture.componentRef.setInput('canCreate', true);
      await fixture.whenStable();
    }

    it('is absent while there is nothing to create', () => {
      expect(fixture.nativeElement.querySelector('.palette-create')).toBeNull();
    });

    it('comes last, quoting what was typed', async () => {
      await offerCreation('migrer la base');

      const rows = options();
      expect(rows).toHaveLength(3);
      expect(rows[2].classList).toContain('palette-create');
      expect(rows[2].textContent).toContain('migrer la base');
    });

    it('replaces the empty state rather than sitting next to it', async () => {
      fixture.componentRef.setInput('results', []);
      await offerCreation('migrer la base');

      expect(fixture.nativeElement.querySelector('.palette-empty')).toBeNull();
      expect(fixture.nativeElement.querySelector('.palette-create')).not.toBeNull();
    });

    it('is chosen like any other option', async () => {
      await offerCreation('migrer la base');
      let index: number | undefined;
      let chosen = 0;
      fixture.componentInstance.highlightSet.subscribe((value) => (index = value));
      fixture.componentInstance.chosen.subscribe(() => (chosen += 1));

      (fixture.nativeElement.querySelector('.palette-create button') as HTMLButtonElement).click();
      await fixture.whenStable();

      expect(index).toBe(2);
      expect(chosen).toBe(1);
    });

    /** Nothing to open: Enter falls back on creating, which is what the row is for. */
    it('is created by Enter rather than opened', async () => {
      await offerCreation('migrer la base');
      fixture.componentRef.setInput('highlighted', 2);
      await fixture.whenStable();
      let opened = 0;
      let chosen = 0;
      fixture.componentInstance.openRequested.subscribe(() => (opened += 1));
      fixture.componentInstance.chosen.subscribe(() => (chosen += 1));

      await press('Enter');

      expect(opened).toBe(0);
      expect(chosen).toBe(1);
    });
  });
});
