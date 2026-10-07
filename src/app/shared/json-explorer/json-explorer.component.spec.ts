import { ComponentFixture, TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';
import { JsonEntry, Span } from '@core/model/json.model';
import { provideTranslocoTesting } from '@testing/provide-transloco-testing';
import { JSON_LINES, JSON_TEXT, jsonView } from '@testing/json-view.fixture';
import { JsonExplorerComponent } from './json-explorer.component';

describe('JsonExplorerComponent', () => {
  let fixture: ComponentFixture<JsonExplorerComponent>;
  let events: {
    search: string[];
    steps: number[];
    toggled: JsonEntry[];
    selected: string[];
    openedAll: number;
    copied: string[];
    shown: Span[];
    treeProposed: number;
  };

  beforeEach(async () => {
    TestBed.configureTestingModule({
      imports: [JsonExplorerComponent],
      providers: [provideTranslocoTesting()],
    });
    fixture = TestBed.createComponent(JsonExplorerComponent);
    fixture.componentRef.setInput('view', jsonView());
    fixture.componentRef.setInput('mode', 'graph');
    fixture.componentRef.setInput('text', JSON_TEXT);
    events = {
      search: [],
      steps: [],
      toggled: [],
      selected: [],
      openedAll: 0,
      copied: [],
      shown: [],
      treeProposed: 0,
    };
    const explorer = fixture.componentInstance;
    explorer.searchChanged.subscribe((value) => events.search.push(value));
    explorer.stepped.subscribe((step) => events.steps.push(step));
    explorer.toggled.subscribe((entry) => events.toggled.push(entry));
    explorer.selected.subscribe((path) => events.selected.push(path));
    explorer.openedAll.subscribe(() => events.openedAll++);
    explorer.copied.subscribe((text) => events.copied.push(text));
    explorer.shownInCode.subscribe((span) => events.shown.push(span));
    explorer.treeProposed.subscribe(() => events.treeProposed++);
    document.body.appendChild(fixture.nativeElement);
    fixture.autoDetectChanges();
    await fixture.whenStable();
  });

  function query<T extends HTMLElement = HTMLElement>(testid: string): T {
    return fixture.nativeElement.querySelector(`[data-testid="${testid}"]`);
  }

  function text(testid: string): string {
    return query(testid).textContent!.replace(/\s+/g, ' ').trim();
  }

  function stats(): string[] {
    return [...fixture.nativeElement.querySelectorAll('.stats dd')].map((dd: Element) =>
      dd.textContent!.trim(),
    );
  }

  async function set(name: string, value: unknown): Promise<void> {
    fixture.componentRef.setInput(name, value);
    await fixture.whenStable();
  }

  describe('the selection', () => {
    it('is the root before anything is chosen', () => {
      expect(text('json-selected-path')).toBe('$');
      expect(text('json-selected-kind')).toBe('Objet · 2 clés');
    });

    it('shows a node: its path, its kind and size, and a preview', async () => {
      await set('selectedPath', '$.data.lines');

      expect(text('json-selected-path')).toBe('$.data.lines');
      expect(text('json-selected-kind')).toBe('Tableau · 1 élément');
      expect(fixture.nativeElement.querySelector('.selected-preview').textContent).toBe('[\n  { … }\n]');
    });

    it('shows a value in a row', async () => {
      await set('selectedPath', '$.id');

      expect(text('json-selected-kind')).toBe('Chaîne');
      expect(fixture.nativeElement.querySelector('.selected-preview').textContent).toBe('"evt"');
    });

    it('shows a line only the tree lists', async () => {
      const tree = [
        ...JSON_LINES,
        { ...JSON_LINES[1]!, path: '$.late', kind: 'boolean' as const, value: 'true' },
      ];
      await set('view', jsonView({ tree }));
      await set('selectedPath', '$.late');

      expect(text('json-selected-kind')).toBe('Booléen');
    });

    it('copies the path and the value as it is written, and goes to it in the code', async () => {
      await set('selectedPath', '$.data');

      query('json-copy-path').click();
      query('json-copy-value').click();
      query('json-show-in-code').click();

      expect(events.copied).toEqual(['$.data', '{"lines":[{"a":1}]}']);
      expect(events.shown).toEqual([{ start: 19, end: 38 }]);
    });

    it('leads back to the root through its breadcrumb', async () => {
      await set('selectedPath', '$.data.lines[0].a');

      expect(text('json-crumbs')).toBe('$ › data › lines › [0] › a');
      query('json-crumbs').querySelector<HTMLElement>('.crumb')!.click();
      expect(events.selected).toEqual(['$']);
    });

    it('shows nothing for a path no longer drawn', async () => {
      await set('selectedPath', '$.gone');

      expect(query('json-selected-path')).toBeNull();
    });
  });

  it('counts the document, its size written the way the language writes numbers', async () => {
    expect(stats()).toEqual(['3', '3', '39 o']);

    await set('view', jsonView({ stats: { keys: 1, depth: 0, bytes: 1843 } }));
    expect(stats()[2]).toBe('1,8 Ko');

    await set('locale', 'en');
    await set('view', jsonView({ stats: { keys: 1, depth: 0, bytes: 3 * 1024 * 1024 } }));
    expect(stats()[2]).toBe('3 Mo');
  });

  describe('the search', () => {
    it('hands what is typed over, and steps on Enter, back on Shift+Enter', () => {
      const field = query<HTMLInputElement>('json-search');
      field.value = 'evt';
      field.dispatchEvent(new Event('input'));
      field.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', bubbles: true }));
      field.dispatchEvent(new KeyboardEvent('keydown', { key: 'Enter', shiftKey: true, bubbles: true }));

      expect(events.search).toEqual(['evt']);
      expect(events.steps).toEqual([1, -1]);
    });

    it('clears on Escape, which then goes no further', () => {
      const field = query<HTMLInputElement>('json-search');
      field.value = 'evt';
      const escape = new KeyboardEvent('keydown', { key: 'Escape', bubbles: true, cancelable: true });
      field.dispatchEvent(escape);

      expect(events.search).toEqual(['']);
      expect(escape.defaultPrevented).toBe(true);

      field.value = '';
      const second = new KeyboardEvent('keydown', { key: 'Escape', bubbles: true, cancelable: true });
      field.dispatchEvent(second);
      expect(second.defaultPrevented).toBe(false);
    });

    it('counts the matches, and says which one it is on', async () => {
      await set('search', 'a');
      await set('view', jsonView({ matchCount: 3 }));
      expect(text('json-match-count')).toBe('3');

      await set('matchIndex', 1);
      expect(text('json-match-count')).toBe('2 / 3');

      query('json-match-next').click();
      query('json-match-previous').click();
      expect(events.steps).toEqual([1, -1]);
    });
  });

  it('opens everything on asking, and fits the graph', () => {
    query('json-open-all').click();
    query('json-fit').click();

    expect(events.openedAll).toBe(1);
  });

  it('turns a row into an entry to open or close', () => {
    fixture.nativeElement.querySelector('[data-testid="json-row"][data-path="$.data"]').click();

    expect(events.toggled).toEqual([{ path: '$.data', opens: true, open: true }]);
  });

  it('shows the tree instead, which toggles its lines', async () => {
    await set('mode', 'tree');

    expect(query('json-graph')).toBeNull();
    expect(query('json-fit')).toBeNull();
    fixture.nativeElement.querySelector('[data-testid="json-line"][data-path="$.data"]').click();
    expect(events.toggled.map((entry) => entry.path)).toEqual(['$.data']);
  });

  it('proposes the tree when the graph had to fold arrays', async () => {
    await set('view', jsonView({ graph: { ...jsonView().graph, folded: true } }));

    query('json-folded').querySelector('button')!.click();

    expect(events.treeProposed).toBe(1);
  });

  it('says where an invalid document breaks, and goes there', async () => {
    await set(
      'view',
      jsonView({
        error: { reason: 'unexpectedEnd', line: 2, column: 4, offset: 12 },
        graph: { nodes: [], width: 0, height: 0, folded: false },
      }),
    );

    expect(text('json-invalid')).toContain(
      'JSON invalide ligne 2, colonne 4 : le document s’arrête trop tôt.',
    );
    query('json-error-show').click();
    expect(events.shown).toEqual([{ start: 12, end: 12 }]);

    await set('withCode', false);
    expect(query('json-error-show')).toBeNull();
  });

  it('offers no way to the code where there is none beside it', async () => {
    await set('withCode', false);
    await set('selectedPath', '$.data');

    expect(query('json-copy-value')).not.toBeNull();
    expect(query('json-show-in-code')).toBeNull();
  });
});
