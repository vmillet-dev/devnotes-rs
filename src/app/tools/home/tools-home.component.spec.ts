import { ComponentFixture, TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';
import { SettingsStore } from '@core/services/settings/settings.store';
import { TOOL_CATALOGUE } from '@core/services/tools/tool.model';
import { ToolsStore } from '@core/services/tools/tools.store';
import { FAKE_TOOLS } from '@testing/tool-catalogue.fixture';
import { provideAppTesting } from '@testing/testing.providers';
import { ToolsHomeComponent } from './tools-home.component';

describe('ToolsHomeComponent', () => {
  let fixture: ComponentFixture<ToolsHomeComponent>;
  let store: ToolsStore;

  beforeEach(async () => {
    TestBed.resetTestingModule();
    localStorage.clear();
    TestBed.configureTestingModule({
      imports: [ToolsHomeComponent],
      providers: [provideAppTesting(), { provide: TOOL_CATALOGUE, useValue: FAKE_TOOLS }],
    });
    store = TestBed.inject(ToolsStore);
    fixture = TestBed.createComponent(ToolsHomeComponent);
    document.body.appendChild(fixture.nativeElement);
    fixture.autoDetectChanges();
    await fixture.whenStable();
  });

  const all = (selector: string): HTMLElement[] => [...fixture.nativeElement.querySelectorAll(selector)];
  const panels = () => all('[data-testid="tools-panel"]').map((panel) => panel.dataset['category']);
  const entries = () => all('[data-testid="tools-entry"]').map((entry) => entry.dataset['tool']);

  it('counts the tools, and says none of them reaches the network', () => {
    expect(fixture.nativeElement.querySelector('[data-testid="tools-count"]').textContent.trim()).toBe(
      '3 utilitaires, tous hors ligne',
    );
    expect(fixture.nativeElement.querySelector('[data-testid="tools-promise"]').textContent).toContain(
      'réseau',
    );
  });

  /** A promise about every tool: above them all, not a block at the end that reads as one more. */
  it('makes the promise a note above the panels, and leaves no block among them', () => {
    const promise: HTMLElement = fixture.nativeElement.querySelector('[data-testid="tools-promise"]');
    const panels: HTMLElement = fixture.nativeElement.querySelector('.panels');

    expect(promise.getAttribute('role')).toBe('note');
    expect(promise.textContent).toContain('Tout reste sur cette machine');
    expect(panels.contains(promise)).toBe(false);
    expect(promise.compareDocumentPosition(panels) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    expect(fixture.nativeElement.querySelectorAll('[data-testid="tools-promise"]')).toHaveLength(1);
  });

  it('lays the tools out in their category panels, in the categories order', () => {
    expect(panels()).toEqual(['text', 'crypto']);
    expect(entries()).toEqual(['case', 'line-breaks', 'hash']);
  });

  it('shows one category when the rail chose it', async () => {
    store.home('crypto');
    await fixture.whenStable();

    expect(panels()).toEqual(['crypto']);
  });

  /**
   * The rail is what says which category is showing. Hidden, the choice is kept — and said
   * in the top bar with the way back, or the catalogue would look as if it had lost tools.
   */
  it('says the category it is held to while the rail is hidden, with the way back', async () => {
    const settings = TestBed.inject(SettingsStore);
    const shown = () => fixture.nativeElement.querySelector('[data-testid="tools-category-shown"]');
    store.home('crypto');
    await fixture.whenStable();
    expect(shown()).toBeNull();

    (fixture.nativeElement.querySelector('[data-testid="tools-rail-toggle"]') as HTMLElement).click();
    await fixture.whenStable();
    expect(settings.showLibraryRail()).toBe(false);
    expect(panels()).toEqual(['crypto']);
    expect(shown().textContent).toContain('Crypto');
    expect(fixture.nativeElement.querySelector('[data-testid="tools-count"]')).toBeNull();

    // A search looks through every category: nothing is held back, so nothing is said.
    store.setSearch('case');
    await fixture.whenStable();
    expect(shown()).toBeNull();
    store.setSearch('');
    await fixture.whenStable();

    (fixture.nativeElement.querySelector('[data-testid="tools-category-clear"]') as HTMLElement).click();
    await fixture.whenStable();
    expect(panels()).toEqual(['text', 'crypto']);
    expect(shown()).toBeNull();
  });

  it('searches the keywords too, across every category', async () => {
    store.home('crypto');
    store.setSearch('CAMEL');
    await fixture.whenStable();

    expect(entries()).toEqual(['case']);
  });

  it('says so when nothing matches', async () => {
    store.setSearch('nothing like it');
    await fixture.whenStable();

    expect(entries()).toEqual([]);
    expect(fixture.nativeElement.querySelector('[data-testid="tools-no-match"]').textContent).toContain(
      'nothing like it',
    );
  });

  it('writes the search as it is typed', async () => {
    const field: HTMLInputElement = fixture.nativeElement.querySelector('[data-testid="tools-search"]');
    field.value = 'sha';
    field.dispatchEvent(new Event('input'));
    await fixture.whenStable();

    expect(store.search()).toBe('sha');
    expect(entries()).toEqual(['hash']);
  });

  it('offers the recent tools, and when they were opened', async () => {
    store.open('hash');
    store.home();
    await fixture.whenStable();

    const recent = fixture.nativeElement.querySelector('[data-testid="tools-recent"]') as HTMLButtonElement;
    expect(recent.dataset['tool']).toBe('hash');
    expect(recent.textContent).toContain("à l'instant");
  });

  it('opens a tool from its panel', () => {
    fixture.nativeElement.querySelector('[data-tool="line-breaks"]').click();

    expect(store.openId()).toBe('line-breaks');
  });

  it('takes the search field when Ctrl+Shift+T asks for it', async () => {
    store.requestSearch();
    await fixture.whenStable();

    expect(document.activeElement?.getAttribute('data-testid')).toBe('tools-search');
    expect(store.searchWanted()).toBe(false);
  });
});
