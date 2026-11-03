import { ComponentFixture, TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';
import { FacetCount, NoteKind } from '@core/model/note.model';
import { provideTranslocoTesting } from '@testing/provide-transloco-testing';
import { KindRailComponent } from './kind-rail.component';

const COUNTS: FacetCount<NoteKind>[] = [
  { value: 'snippet', count: 24 },
  { value: 'note', count: 9 },
  { value: 'checklist', count: 0 },
];

describe('KindRailComponent', () => {
  let fixture: ComponentFixture<KindRailComponent>;

  function chip(kind: NoteKind | 'all'): HTMLButtonElement {
    return fixture.nativeElement.querySelector(`[data-testid="kind-chip-${kind}"]`);
  }

  async function show(counts: FacetCount<NoteKind>[], active: NoteKind[] = []): Promise<void> {
    fixture.componentRef.setInput('counts', counts);
    fixture.componentRef.setInput('activeKinds', new Set(active));
    await fixture.whenStable();
  }

  beforeEach(() => {
    TestBed.configureTestingModule({
      imports: [KindRailComponent],
      providers: [provideTranslocoTesting()],
    });
    fixture = TestBed.createComponent(KindRailComponent);
    fixture.componentRef.setInput('counts', []);
    fixture.componentRef.setInput('activeKinds', new Set());
    fixture.autoDetectChanges();
  });

  it('renders nothing for an empty space', () => {
    expect(fixture.nativeElement.querySelector('[data-testid="kind-rail"]')).toBeNull();
  });

  /** The mockup's row: "Tous 33 · Snippets 24 · Notes 9 · Tâches 0". */
  it('counts every kind, and all of them together first', async () => {
    await show(COUNTS);

    const chips: HTMLButtonElement[] = [...fixture.nativeElement.querySelectorAll('.kind-chip')];
    const read = (button: HTMLButtonElement): string =>
      [...button.querySelectorAll('span')].map((span) => span.textContent?.trim()).join(' ');
    expect(chips.map(read)).toEqual(['Tous 33', 'Snippets 24', 'Notes 9', 'Tâches 0']);
    expect(chip('all').getAttribute('aria-pressed')).toBe('true');
  });

  it('presses the kinds chosen, and not "All" while one is', async () => {
    await show(COUNTS, ['note']);

    expect(chip('note').getAttribute('aria-pressed')).toBe('true');
    expect(chip('snippet').getAttribute('aria-pressed')).toBe('false');
    expect(chip('all').getAttribute('aria-pressed')).toBe('false');
  });

  /** Nothing to find behind it; pressed from another space, it must still be let go of. */
  it('offers an empty kind only while it is pressed', async () => {
    await show(COUNTS);
    expect(chip('checklist').disabled).toBe(true);

    await show(COUNTS, ['checklist']);
    expect(chip('checklist').disabled).toBe(false);
  });

  it('emits the kind a click landed on, and "All" apart', async () => {
    await show(COUNTS, ['note']);
    const toggled: NoteKind[] = [];
    let all = 0;
    fixture.componentInstance.kindToggled.subscribe((kind) => toggled.push(kind));
    fixture.componentInstance.allChosen.subscribe(() => (all += 1));

    chip('snippet').click();
    chip('all').click();

    expect(toggled).toEqual(['snippet']);
    expect(all).toBe(1);
  });

  it('stays while a kind is pressed, even in a space holding none', async () => {
    await show(
      COUNTS.map((counted) => ({ ...counted, count: 0 })),
      ['note'],
    );

    expect(fixture.nativeElement.querySelector('[data-testid="kind-rail"]')).not.toBeNull();
  });
});
