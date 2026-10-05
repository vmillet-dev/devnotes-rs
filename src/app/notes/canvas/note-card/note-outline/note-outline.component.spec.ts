import { ComponentFixture, TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';
import { OutlineLine } from '@core/model/note.model';
import { NoteOutlineComponent } from './note-outline.component';

describe('NoteOutlineComponent', () => {
  let fixture: ComponentFixture<NoteOutlineComponent>;

  async function draw(lines: OutlineLine[]): Promise<HTMLElement> {
    fixture.componentRef.setInput('lines', lines);
    await fixture.whenStable();
    return fixture.nativeElement as HTMLElement;
  }

  beforeEach(() => {
    TestBed.configureTestingModule({ imports: [NoteOutlineComponent] });
    fixture = TestBed.createComponent(NoteOutlineComponent);
  });

  it('draws a heading apart from the items under it, each with its marker', async () => {
    const outline = await draw([
      { kind: 'heading', text: 'Décisions' },
      { kind: 'item', text: 'Bascule le 14/10', depth: 0, marker: '•' },
      { kind: 'item', text: 'Réplica d’abord', depth: 1, marker: '2.' },
    ]);

    expect(outline.querySelector('.outline-heading')?.textContent).toBe('Décisions');
    const items = [...outline.querySelectorAll<HTMLElement>('.outline-item')];
    expect(items.map((item) => item.textContent)).toEqual(['•Bascule le 14/10', '2.Réplica d’abord']);
    expect(items[1]?.style.getPropertyValue('--depth')).toBe('1');
  });

  it('says whether a task is done, and keeps code as written', async () => {
    const outline = await draw([
      { kind: 'task', text: 'ship it', depth: 0, done: true },
      { kind: 'code', text: 'SELECT 1;\n  -- next' },
      { kind: 'text', text: 'Done.' },
    ]);

    expect(outline.querySelector('.outline-item')?.textContent).toBe('☑ship it');
    expect(outline.querySelector('.outline-code')?.textContent).toBe('SELECT 1;\n  -- next');
    expect(outline.querySelector('.outline-text')?.textContent).toBe('Done.');
  });

  it('keeps every line of a long list whole, the text of an item in an element of its own', async () => {
    const outline = await draw(
      Array.from({ length: 14 }, (_, at) => ({
        kind: 'item' as const,
        text: `Point ${at + 1}, long enough to need an ellipsis on a card of the canvas`,
        depth: at % 2,
        marker: '•',
      })),
    );

    const items = [...outline.querySelectorAll<HTMLElement>('.outline-item')];
    expect(items).toHaveLength(14);
    for (const [at, item] of items.entries()) {
      expect(item.querySelector('.outline-item-text')?.textContent).toBe(
        `Point ${at + 1}, long enough to need an ellipsis on a card of the canvas`,
      );
    }
  });
});
