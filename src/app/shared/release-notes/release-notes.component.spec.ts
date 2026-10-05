import { ComponentFixture, TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';
import type { ChangelogSection } from '@core/ipc/bindings';
import { ReleaseNotesComponent } from './release-notes.component';

describe('ReleaseNotesComponent', () => {
  let fixture: ComponentFixture<ReleaseNotesComponent>;

  async function draw(sections: readonly ChangelogSection[]): Promise<HTMLElement> {
    fixture.componentRef.setInput('sections', sections);
    await fixture.whenStable();
    return fixture.nativeElement as HTMLElement;
  }

  beforeEach(() => {
    TestBed.configureTestingModule({ imports: [ReleaseNotesComponent] });
    fixture = TestBed.createComponent(ReleaseNotesComponent);
  });

  it('draws each category under its heading, an entry per bullet', async () => {
    const notes = await draw([
      {
        title: '✨ Added',
        items: [[{ kind: 'plain', text: 'Samples' }], [{ kind: 'plain', text: 'A diff' }]],
      },
      { title: '🐛 Fixed', items: [[{ kind: 'plain', text: 'The rail' }]] },
    ]);

    expect([...notes.querySelectorAll('h4')].map((heading) => heading.textContent)).toEqual([
      '✨ Added',
      '🐛 Fixed',
    ]);
    expect([...notes.querySelectorAll('ul')].map((list) => list.children.length)).toEqual([2, 1]);
  });

  it('draws emphasis and code as elements, never as markup to interpret', async () => {
    const notes = await draw([
      {
        title: '',
        items: [
          [
            { kind: 'strong', text: 'Todo lists.' },
            { kind: 'plain', text: ' Fill a ' },
            { kind: 'code', text: '<b>{{field}}</b>' },
          ],
        ],
      },
    ]);

    expect(notes.querySelector('h4')).toBeNull();
    expect(notes.querySelector('li strong')?.textContent).toBe('Todo lists.');
    expect(notes.querySelector('li code')?.textContent).toBe('<b>{{field}}</b>');
    expect(notes.querySelector('li b')).toBeNull();
  });
});
