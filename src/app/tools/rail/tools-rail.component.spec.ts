import { ComponentFixture, TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';
import { AreaStore } from '@core/services/areas/area.store';
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
    store.open('slug');
    await fixture.whenStable();

    expect(ids('tools-rail-tool', 'tool')).toEqual(['case', 'slug']);
    expect(link('[data-tool="slug"]').getAttribute('aria-current')).toBe('page');
    expect(ids('tools-rail-recent', 'tool')).toEqual(['hash']);

    link('[data-testid="tools-rail-recent"]').click();
    expect(store.openId()).toBe('hash');
  });

  it('is headed by the switch between areas', () => {
    link('[data-area="notes"]').click();

    expect(TestBed.inject(AreaStore).current()).toBe('notes');
  });
});
