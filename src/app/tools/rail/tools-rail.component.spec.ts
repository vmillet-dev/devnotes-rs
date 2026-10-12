import { ComponentFixture, TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';
import { AreaStore } from '@core/services/areas/area.store';
import { RAIL_WIDTH } from '@core/services/settings/app-settings.model';
import { SettingsStore } from '@core/services/settings/settings.store';
import { TOOL_CATALOGUE } from '@core/services/tools/tool.model';
import { ToolsStore } from '@core/services/tools/tools.store';
import { FAKE_TOOLS } from '@testing/tool-catalogue.fixture';
import { provideAppTesting } from '@testing/testing.providers';
import { ToolsRailComponent } from './tools-rail.component';

describe('ToolsRailComponent', () => {
  let fixture: ComponentFixture<ToolsRailComponent>;
  let store: ToolsStore;

  beforeEach(async () => {
    TestBed.resetTestingModule();
    localStorage.clear();
    TestBed.configureTestingModule({
      imports: [ToolsRailComponent],
      providers: [provideAppTesting(), { provide: TOOL_CATALOGUE, useValue: FAKE_TOOLS }],
    });
    store = TestBed.inject(ToolsStore);
    fixture = TestBed.createComponent(ToolsRailComponent);
    fixture.autoDetectChanges();
    await fixture.whenStable();
  });

  const ids = (testid: string, attribute: string): string[] =>
    [...fixture.nativeElement.querySelectorAll(`[data-testid="${testid}"]`)].map(
      (element: HTMLElement) => element.dataset[attribute]!,
    );
  const link = (selector: string): HTMLButtonElement => fixture.nativeElement.querySelector(selector);

  /** Only the notes' rail had an edge: this one took the width and could not change it. */
  it("is as wide as the other areas' rails, and is widened from its own edge", async () => {
    const settings = TestBed.inject(SettingsStore);
    const frame: HTMLElement = fixture.nativeElement.querySelector('app-rail-frame');
    settings.libraryRailWidth.write(240);
    await fixture.whenStable();
    expect(frame.style.width).toBe('240px');

    const edge: HTMLElement = fixture.nativeElement.querySelector('[data-testid="tools-rail-edge"]');
    edge.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowLeft', bubbles: true }));
    await fixture.whenStable();

    expect(settings.libraryRailWidth()).toBe(224);
    expect(edge.getAttribute('aria-valuemin')).toBe(String(RAIL_WIDTH.min));
  });

  it('lists the categories that hold something, with their counts', () => {
    expect(ids('tools-rail-category', 'category')).toEqual(['text', 'crypto']);
    expect(link('[data-testid="tools-rail-all"]').textContent).toContain('3');
    expect(link('[data-category="text"]').textContent).toContain('2');
    expect(link('[data-testid="tools-rail-all"]').getAttribute('aria-current')).toBe('page');
  });

  it('narrows the home to a category, and widens it again', async () => {
    link('[data-category="crypto"]').click();
    await fixture.whenStable();
    expect(store.category()).toBe('crypto');
    expect(link('[data-category="crypto"]').getAttribute('aria-current')).toBe('page');

    link('[data-testid="tools-rail-all"]').click();
    expect(store.category()).toBeNull();
  });

  it("shows a tool's category, and the recent tools that are not in it", async () => {
    store.open('hash');
    store.open('line-breaks');
    await fixture.whenStable();

    expect(ids('tools-rail-tool', 'tool')).toEqual(['case', 'line-breaks']);
    expect(link('[data-tool="line-breaks"]').getAttribute('aria-current')).toBe('page');
    expect(ids('tools-rail-recent', 'tool')).toEqual(['hash']);

    link('[data-testid="tools-rail-recent"]').click();
    expect(store.openId()).toBe('hash');
  });

  /** Three things told apart: « Tous les outils », the section's tools, then the recents. */
  it('heads the section and the recents apart, and highlights the open tool alone', async () => {
    store.open('hash');
    store.open('line-breaks');
    await fixture.whenStable();

    const rail: HTMLElement = fixture.nativeElement.querySelector('[data-testid="tools-rail"]');
    const order = [...rail.querySelectorAll('[data-testid^="tools-rail-"], hr')].map(
      (element) => element.getAttribute('data-testid') ?? element.localName,
    );
    expect(order).toEqual([
      'tools-rail-all',
      'tools-rail-heading',
      'tools-rail-tool',
      'tools-rail-tool',
      'hr',
      'tools-rail-heading',
      'tools-rail-recent',
    ]);
    // A heading is a label, not a row to click.
    expect([...rail.querySelectorAll('[data-testid="tools-rail-heading"]')].map((h) => h.localName)).toEqual([
      'h2',
      'h2',
    ]);
    // The area switch above says which area is on screen; among the rows, one says which tool.
    const current = rail.querySelectorAll('[data-testid^="tools-rail-"][aria-current="page"]');
    expect([...current].map((row) => row.getAttribute('data-tool'))).toEqual(['line-breaks']);
    expect(rail.querySelectorAll('.rail-link.on')).toHaveLength(1);
  });

  /**
   * Five on the home, four or fewer in the rail of an open tool: the rail drops the recents its
   * section already lists, and only five were kept — the open tool always one of them.
   */
  it('still shows five recents when some of the last opened are listed above them', async () => {
    const elsewhere = ['a', 'b', 'c', 'd', 'e', 'f'].map((id) => ({
      id,
      category: 'crypto' as const,
      keywords: [],
      load: FAKE_TOOLS[0]!.load,
    }));
    TestBed.resetTestingModule();
    localStorage.clear();
    TestBed.configureTestingModule({
      imports: [ToolsRailComponent],
      providers: [
        provideAppTesting(),
        {
          provide: TOOL_CATALOGUE,
          useValue: [...FAKE_TOOLS.filter((tool) => tool.category === 'text'), ...elsewhere],
        },
      ],
    });
    store = TestBed.inject(ToolsStore);
    fixture = TestBed.createComponent(ToolsRailComponent);
    fixture.autoDetectChanges();

    for (const id of ['a', 'b', 'c', 'line-breaks', 'd', 'e', 'f', 'case']) store.open(id);
    await fixture.whenStable();

    // The two tools of the open one's section are above; the five others opened last follow.
    expect(ids('tools-rail-tool', 'tool')).toEqual(['case', 'line-breaks']);
    expect(ids('tools-rail-recent', 'tool')).toEqual(['f', 'e', 'd', 'c', 'b']);
  });

  it('is headed by the switch between areas', () => {
    link('[data-area="notes"]').click();

    expect(TestBed.inject(AreaStore).current()).toBe('notes');
  });
});
