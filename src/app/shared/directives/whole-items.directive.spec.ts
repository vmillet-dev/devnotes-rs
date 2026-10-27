import { Component, signal } from '@angular/core';
import { TestBed } from '@angular/core/testing';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { WholeItemsDirective, wholeCount } from './whole-items.directive';

describe('wholeCount', () => {
  const box = { top: 100, right: 200 };

  it('counts the items standing whole on the first line', () => {
    expect(
      wholeCount(box, [
        { top: 100, right: 150 },
        { top: 100, right: 199.6 },
      ]),
    ).toBe(2);
  });

  it('stops at the first one wrapped onto the next line', () => {
    expect(
      wholeCount(box, [
        { top: 100, right: 150 },
        { top: 114, right: 140 },
        { top: 100, right: 160 },
      ]),
    ).toBe(1);
  });

  it('leaves out a first item wider than the row, rather than cut it', () => {
    expect(wholeCount(box, [{ top: 100, right: 260 }])).toBe(0);
  });
});

@Component({
  imports: [WholeItemsDirective],
  template: `<span [appWholeItems]="tags().length" #row="appWholeItems">
      @for (tag of tags(); track tag) {
        <span [attr.data-right]="tag">#{{ tag }}</span>
      }
    </span>
    <b>{{ row.shown() }}</b>`,
})
class Host {
  readonly tags = signal(['120', '180', '240']);
}

describe('WholeItemsDirective', () => {
  afterEach(() => vi.unstubAllGlobals());

  it('hides what does not stand whole and says how many do', async () => {
    let resized: (entries: { target: Element }[]) => void = () => undefined;
    vi.stubGlobal(
      'ResizeObserver',
      class {
        observe = vi.fn();
        unobserve = vi.fn();

        constructor(callback: (entries: { target: Element }[]) => void) {
          resized = callback;
        }
      },
    );
    const rect = (element: Element, right: number) =>
      vi.spyOn(element, 'getBoundingClientRect').mockReturnValue({ top: 0, right } as DOMRect);

    const fixture = TestBed.createComponent(Host);
    fixture.autoDetectChanges();
    await fixture.whenStable();
    const row = fixture.nativeElement.querySelector('span') as HTMLElement;
    vi.spyOn(row, 'getClientRects').mockReturnValue([{}] as unknown as DOMRectList);
    rect(row, 200);
    const items = [...row.children] as HTMLElement[];
    items.forEach((item) => rect(item, Number(item.dataset['right'])));

    resized([{ target: row }]);
    await fixture.whenStable();

    expect(items.map((item) => item.style.visibility)).toEqual(['', '', 'hidden']);
    expect(fixture.nativeElement.querySelector('b').textContent).toBe('2');

    fixture.destroy();
  });

  it('shows every item where nothing can be measured', async () => {
    const fixture = TestBed.createComponent(Host);
    fixture.autoDetectChanges();
    await fixture.whenStable();

    expect(fixture.nativeElement.querySelector('b').textContent).toBe('3');
  });
});
