import { ChangeDetectionStrategy, Component, signal } from '@angular/core';
import { ComponentFixture, TestBed } from '@angular/core/testing';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { TwoRowsDirective } from './two-rows.directive';

@Component({
  selector: 'app-two-rows-host',
  imports: [TwoRowsDirective],
  changeDetection: ChangeDetectionStrategy.OnPush,
  template: `
    <div id="bar" appTwoRows #rows="appTwoRows">
      <div data-group>
        <button type="button" id="rail">Rail</button>
        @if (folderShown()) {
          <button type="button" id="folder">Dossier</button>
        }
      </div>
      <span id="search" data-floor="50">Rechercher…</span>
    </div>
    <p id="answer">{{ rows.twoRows() }}</p>
  `,
})
class TwoRowsHostComponent {
  readonly folderShown = signal(false);
}

/** Stands in for the browser's: jsdom lays nothing out, so the spec says what changed. */
class FakeResizeObserver {
  static last: FakeResizeObserver | null = null;
  readonly observed = new Set<Element>();
  disconnected = 0;

  constructor(readonly callback: () => void) {
    FakeResizeObserver.last = this;
  }

  observe(target: Element): void {
    this.observed.add(target);
  }

  disconnect(): void {
    this.observed.clear();
    this.disconnected += 1;
  }
}

describe('TwoRowsDirective', () => {
  let fixture: ComponentFixture<TwoRowsHostComponent>;
  let barWidth: number;
  /** As drawn; the search's 999 is what its floor of 50 stands in for. */
  const widths: Record<string, number> = { rail: 100, folder: 40, search: 999 };

  const bar = (): HTMLElement => fixture.nativeElement.querySelector('#bar');
  const answer = (): string => fixture.nativeElement.querySelector('#answer').textContent.trim();

  async function resized(width: number): Promise<void> {
    barWidth = width;
    FakeResizeObserver.last!.callback();
    await fixture.whenStable();
  }

  beforeEach(async () => {
    barWidth = 170;
    vi.stubGlobal('ResizeObserver', FakeResizeObserver);
    vi.spyOn(HTMLElement.prototype, 'clientWidth', 'get').mockImplementation(function (this: HTMLElement) {
      return this.id === 'bar' ? barWidth : 0;
    });
    vi.spyOn(HTMLElement.prototype, 'getBoundingClientRect').mockImplementation(function (this: HTMLElement) {
      return { width: widths[this.id] ?? 0 } as DOMRect;
    });
    const real = window.getComputedStyle.bind(window);
    vi.spyOn(window, 'getComputedStyle').mockImplementation((element, pseudo) =>
      (element as HTMLElement).id === 'bar'
        ? ({ columnGap: '10px', paddingLeft: '5px', paddingRight: '5px' } as CSSStyleDeclaration)
        : real(element, pseudo),
    );

    TestBed.configureTestingModule({ imports: [TwoRowsHostComponent] });
    fixture = TestBed.createComponent(TwoRowsHostComponent);
    fixture.autoDetectChanges();
    await fixture.whenStable();
  });

  afterEach(() => {
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });

  /** 100 + the search's floor of 50 + one gap of 10 is 160, inside 170 less 10 of padding. */
  it('keeps one row while every control holds, the narrowing ones at their floor', () => {
    expect(answer()).toBe('false');
    expect(bar().classList).not.toContain('two-rows');
  });

  it('takes two rows once they do not, and one again once they do', async () => {
    await resized(169);
    expect(answer()).toBe('true');
    expect(bar().classList).toContain('two-rows');

    await resized(170);
    expect(answer()).toBe('false');
  });

  it('watches a control that appears, inside a group, and counts it', async () => {
    fixture.componentInstance.folderShown.set(true);
    await fixture.whenStable();
    await vi.waitFor(() => expect(answer()).toBe('true'));

    const watched = [...FakeResizeObserver.last!.observed].map((element) => element.id);
    expect(watched).toEqual(['bar', 'rail', 'folder', 'search']);
  });

  it('keeps its answer while the page is hidden', async () => {
    await resized(169);

    await resized(0);

    expect(answer()).toBe('true');
  });

  it('lets go of what it watches with its view', () => {
    const observer = FakeResizeObserver.last!;
    const before = observer.disconnected;

    fixture.destroy();

    expect(observer.disconnected).toBe(before + 1);
  });
});

describe('TwoRowsDirective, without a ResizeObserver', () => {
  it('stays one row', async () => {
    vi.stubGlobal('ResizeObserver', undefined);
    try {
      TestBed.configureTestingModule({ imports: [TwoRowsHostComponent] });
      const fixture = TestBed.createComponent(TwoRowsHostComponent);
      fixture.autoDetectChanges();
      await fixture.whenStable();

      expect(fixture.nativeElement.querySelector('#answer').textContent.trim()).toBe('false');
    } finally {
      vi.unstubAllGlobals();
    }
  });
});
