import { ComponentFixture, TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { HttpCollectionsStore } from '@core/services/http/http-collections.store';
import { HttpHistoryStore } from '@core/services/http/http-history.store';
import { HttpTabsStore } from '@core/services/http/http-tabs.store';
import { RAIL_WIDTH } from '@core/services/settings/app-settings.model';
import { SettingsStore } from '@core/services/settings/settings.store';
import { FakeHttpRepository } from '@testing/fake-http-repository';
import { collection, request, sampleTree } from '@testing/http-tree.fixture';
import { provideAppTesting } from '@testing/testing.providers';
import { HttpRailComponent } from './http-rail.component';

describe('HttpRailComponent', () => {
  let fixture: ComponentFixture<HttpRailComponent>;
  let http: FakeHttpRepository;

  beforeEach(async () => {
    TestBed.resetTestingModule();
    http = new FakeHttpRepository();
    http.tree$ = sampleTree();
    http.seed('Login');
    TestBed.configureTestingModule({
      imports: [HttpRailComponent],
      providers: [provideAppTesting({ httpRepository: http })],
    });
    fixture = TestBed.createComponent(HttpRailComponent);
    fixture.autoDetectChanges();
    await TestBed.inject(HttpCollectionsStore).load();
    await fixture.whenStable();
  });

  const el = <E extends HTMLElement = HTMLElement>(selector: string): E =>
    fixture.nativeElement.querySelector(selector);
  const all = (selector: string): HTMLElement[] => [...fixture.nativeElement.querySelectorAll(selector)];
  const row = (id: string) => el(`[data-testid="http-node"][data-id="${id}"]`);
  const name = (id: string) =>
    el<HTMLButtonElement>(`[data-testid="http-node"][data-id="${id}"] [data-testid="http-node-name"]`);
  const action = async (id: string, which: string) => {
    el<HTMLButtonElement>(
      `[data-testid="http-node"][data-id="${id}"] [data-testid="http-node-menu"]`,
    ).click();
    await fixture.whenStable();
    el<HTMLButtonElement>(`[data-testid="http-node-action"][data-action="${which}"]`).click();
    await fixture.whenStable();
  };
  const type = async (testid: string, text: string, submit: string) => {
    const field = el<HTMLInputElement>(`[data-testid="${testid}"]`);
    field.value = text;
    el<HTMLButtonElement>(`[data-testid="${submit}"]`).click();
    await fixture.whenStable();
  };
  /** jsdom moves no focus on a click: the field is left by the event a browser would fire. */
  const leave = async (testid: string, text: string, to: HTMLElement | null = null) => {
    const field = el<HTMLInputElement>(`[data-testid="${testid}"]`);
    field.value = text;
    field.dispatchEvent(new FocusEvent('focusout', { bubbles: true, relatedTarget: to }));
    await fixture.whenStable();
  };

  it('draws the collections, their folders and requests, a request after its method', () => {
    expect(all('[data-testid="http-node"]').map((node) => node.dataset['id'])).toContain('Old');
    expect(all('[data-testid="http-node-badge"]').map((badge) => badge.textContent?.trim())).toEqual([
      'GET',
      'GET',
      'GET',
      'POST',
      'GQL',
      'WS',
    ]);
    expect(row('Old').style.getPropertyValue('--depth')).toBe('3');
  });

  /**
   * It used to take at least 240px whatever was stored, so the rail jumped on the way in from
   * the notes and back on the way out. Now it is the one width, and it can be dragged here too.
   */
  it("is as wide as the other areas' rails, down to their floor, and is widened from its own edge", async () => {
    const settings = TestBed.inject(SettingsStore);
    const frame = () => el('app-rail-frame');
    expect(settings.libraryRailWidth()).toBe(RAIL_WIDTH.min);
    expect(frame().style.width).toBe(`${RAIL_WIDTH.min}px`);

    el('[data-testid="http-rail-edge"]').dispatchEvent(
      new KeyboardEvent('keydown', { key: 'ArrowRight', bubbles: true }),
    );
    await fixture.whenStable();

    expect(settings.libraryRailWidth()).toBe(RAIL_WIDTH.min + 16);
    expect(frame().style.width).toBe(`${RAIL_WIDTH.min + 16}px`);
  });

  /** What lets a name show at the floor: no room held for a caret a request has not, a short method. */
  it('keeps a request row to its method and its name, whole in its title', async () => {
    http.tree$ = {
      collections: [collection('API', [request('Supprimer la facture 2026', 'API', null, 'DELETE')])],
    };
    await TestBed.inject(HttpCollectionsStore).load();
    await fixture.whenStable();
    const id = 'Supprimer la facture 2026';

    expect(row(id).querySelector('.node-twisty-space')).toBeNull();
    expect(row(id).querySelector('[data-testid="http-node-twisty"]')).toBeNull();
    const badge = row(id).querySelector<HTMLElement>('[data-testid="http-node-badge"]')!;
    expect(badge.textContent?.trim()).toBe('DEL');
    expect(badge.dataset['method']).toBe('DELETE');
    expect(name(id).getAttribute('title')).toBe(id);
  });

  it('folds a folder from its twisty, and opens a request from its row', async () => {
    el<HTMLButtonElement>(
      '[data-testid="http-node"][data-id="Factures"] [data-testid="http-node-twisty"]',
    ).click();
    await fixture.whenStable();
    expect(row('Archives')).toBeNull();

    name('Login').click();
    await vi.waitFor(() => expect(TestBed.inject(HttpTabsStore).activeRequestId()).toBe('Login'));
    await fixture.whenStable();
    expect(name('Login').getAttribute('aria-current')).toBe('true');

    name('Auth').click();
    await fixture.whenStable();
    expect(row('Login')).toBeNull();
  });

  it('creates a collection from the head of the rail', async () => {
    el<HTMLButtonElement>('[data-testid="http-collection-create-open"]').click();
    await fixture.whenStable();
    expect(document.activeElement).toBe(el('[data-testid="http-create-input"]'));

    await type('http-create-input', '  Paiements  ', 'http-create-submit');

    expect(http.callsOf('createCollection')).toEqual([['Paiements']]);
    expect(el('[data-testid="http-create-input"]')).toBeNull();
  });

  /**
   * The « + » of the tabs begins a request a few pixels from this one, the same glyph at the
   * same size: this one draws what it makes, and names it once the rail has the room.
   */
  it('draws the head’s button as what it makes: an icon at the floor, « ＋ Collection » on a wider rail', async () => {
    const button = () => el<HTMLButtonElement>('[data-testid="http-collection-create-open"]');
    expect(button().dataset['worded']).toBe('false');
    expect(button().querySelector('app-icon')).not.toBeNull();
    expect(button().title).toBe('Nouvelle collection');
    expect(button().getAttribute('aria-label')).toBe('Nouvelle collection');

    TestBed.inject(SettingsStore).libraryRailWidth.write(240);
    await fixture.whenStable();

    expect(button().dataset['worded']).toBe('true');
    expect(button().textContent?.trim()).toBe('＋ Collection');
    expect(button().title).toBe('Nouvelle collection');
  });

  /**
   * A field left by a click elsewhere stayed open, unfocused, its text belonging to nothing.
   * Leaving it keeps what was typed, as a file explorer does.
   */
  it('creates what was typed when the field is left, once, and closes an empty one', async () => {
    el<HTMLButtonElement>('[data-testid="http-collection-create-open"]').click();
    await fixture.whenStable();
    const submit = el<HTMLButtonElement>('[data-testid="http-create-submit"]');
    await leave('http-create-input', '  Paiements  ');

    expect(http.callsOf('createCollection')).toEqual([['Paiements']]);
    expect(el('[data-testid="http-create-input"]')).toBeNull();
    // The press that left the field ends on the button it was beside: no second write.
    submit.click();
    await fixture.whenStable();
    expect(http.callsOf('createCollection')).toHaveLength(1);

    el<HTMLButtonElement>('[data-testid="http-collection-create-open"]').click();
    await fixture.whenStable();
    await leave('http-create-input', '   ');
    expect(el('[data-testid="http-create-input"]')).toBeNull();
    expect(http.callsOf('createCollection')).toHaveLength(1);
  });

  it('keeps the same rule under a row: a folder and a request left by a click are created once', async () => {
    await action('Auth', 'newFolder');
    await leave('http-create-input', 'Jetons');
    await action('API', 'newGraphql');
    await leave('http-create-input', 'Produits');

    await vi.waitFor(() => expect(http.callsOf('createRequest')).toHaveLength(1));
    expect(http.callsOf('createFolder')).toEqual([['API', 'Auth', 'Jetons']]);
    expect(http.callsOf('createRequest')[0]?.[0]).toMatchObject({ name: 'Produits', kind: 'graphql' });
    expect(el('[data-testid="http-create-input"]')).toBeNull();
  });

  /** A press on « Créer » is not the field left and then a submit: it is one write. */
  it('does not take a press on the form’s own button for the field left', async () => {
    el<HTMLButtonElement>('[data-testid="http-collection-create-open"]').click();
    await fixture.whenStable();
    const submit = el<HTMLButtonElement>('[data-testid="http-create-submit"]');

    await leave('http-create-input', 'Paiements', submit);
    expect(http.callsOf('createCollection')).toEqual([]);
    expect(el('[data-testid="http-create-input"]')).not.toBeNull();

    submit.click();
    await fixture.whenStable();
    expect(http.callsOf('createCollection')).toEqual([['Paiements']]);

    // From the button to the rest of the window, the form goes with what it holds.
    el<HTMLButtonElement>('[data-testid="http-collection-create-open"]').click();
    await fixture.whenStable();
    el<HTMLInputElement>('[data-testid="http-create-input"]').value = 'Stocks';
    el('[data-testid="http-create-submit"]').dispatchEvent(new FocusEvent('focusout', { bubbles: true }));
    await fixture.whenStable();
    expect(http.callsOf('createCollection')).toEqual([['Paiements'], ['Stocks']]);
  });

  it('renames to what was typed when the field is left, and writes nothing for the same name or none', async () => {
    await action('Login', 'rename');
    await leave('http-rename-input', 'Se connecter');
    expect(http.callsOf('rename')).toEqual([[{ kind: 'request', id: 'Login' }, 'Se connecter']]);
    expect(el('[data-testid="http-rename-input"]')).toBeNull();

    await action('Factures', 'rename');
    await leave('http-rename-input', 'Factures');
    expect(el('[data-testid="http-rename-input"]')).toBeNull();

    await action('Factures', 'rename');
    await leave('http-rename-input', '  ');
    expect(el('[data-testid="http-rename-input"]')).toBeNull();
    expect(http.callsOf('rename')).toHaveLength(1);
  });

  it('cancels on Escape whatever was typed: nothing is written, the tree is as it was', async () => {
    const escape = () => new KeyboardEvent('keydown', { key: 'Escape', bubbles: true });
    const before = all('[data-testid="http-node"]').map((node) => node.dataset['id']);

    el<HTMLButtonElement>('[data-testid="http-collection-create-open"]').click();
    await fixture.whenStable();
    const created = el<HTMLInputElement>('[data-testid="http-create-input"]');
    created.value = 'Paiements';
    created.dispatchEvent(escape());
    await fixture.whenStable();

    await action('Login', 'rename');
    const renamed = el<HTMLInputElement>('[data-testid="http-rename-input"]');
    renamed.value = 'Se connecter';
    renamed.dispatchEvent(escape());
    await fixture.whenStable();

    expect(el('[data-testid="http-create-input"]')).toBeNull();
    expect(el('[data-testid="http-rename-input"]')).toBeNull();
    expect(http.callsOf('createCollection')).toEqual([]);
    expect(http.callsOf('rename')).toEqual([]);
    expect(all('[data-testid="http-node"]').map((node) => node.dataset['id'])).toEqual(before);
  });

  it('creates a folder and a request under the row whose menu asked', async () => {
    await action('Auth', 'newFolder');
    expect(el('[data-testid="http-create-input"]').dataset['kind']).toBe('folder');
    await type('http-create-input', 'Jetons', 'http-create-submit');

    await action('API', 'newHttp');
    await type('http-create-input', 'Santé', 'http-create-submit');

    expect(http.callsOf('createFolder')).toEqual([['API', 'Auth', 'Jetons']]);
    expect(http.callsOf('createRequest')[0]?.[0]).toMatchObject({
      collectionId: 'API',
      folderId: null,
      name: 'Santé',
      kind: 'http',
    });
    await vi.waitFor(() => expect(TestBed.inject(HttpTabsStore).activeRequestId()).toMatch(/^request-/));
  });

  /** « Nouvelle requête » made an HTTP `GET`, whatever was meant: its kind is chosen first. */
  it('offers the three kinds of request in a row’s menu, and creates the one chosen under its name', async () => {
    el<HTMLButtonElement>('[data-testid="http-node"][data-id="Auth"] [data-testid="http-node-menu"]').click();
    await fixture.whenStable();
    expect(
      all('[data-testid="http-node-action"]')
        .map((entry) => entry.dataset['action'])
        .slice(0, 4),
    ).toEqual(['newHttp', 'newGraphql', 'newWebsocket', 'newFolder']);
    el<HTMLButtonElement>('[data-testid="http-node-action"][data-action="newGraphql"]').click();
    await fixture.whenStable();
    await type('http-create-input', 'Produits', 'http-create-submit');

    await action('API', 'newWebsocket');
    await type('http-create-input', 'Flux', 'http-create-submit');

    await vi.waitFor(() => expect(http.callsOf('createRequest')).toHaveLength(2));
    expect(http.callsOf('createRequest').map((call) => call[0])).toMatchObject([
      { collectionId: 'API', folderId: 'Auth', name: 'Produits', kind: 'graphql' },
      { collectionId: 'API', folderId: null, name: 'Flux', kind: 'websocket' },
    ]);
  });

  it('opens a collection’s settings, and has open tabs read what they inherit again once saved', async () => {
    const store = TestBed.inject(HttpCollectionsStore);
    const revision = store.settingsRevision();
    await action('API', 'settings');
    await vi.waitFor(() =>
      expect(document.body.querySelector('[data-testid="http-settings-dialog"]')).not.toBeNull(),
    );

    document.body.querySelector<HTMLButtonElement>('[data-testid="http-settings-cancel"]')!.click();
    await fixture.whenStable();
    expect(document.body.querySelector('[data-testid="http-settings-dialog"]')).toBeNull();
    expect(store.settingsRevision()).toBe(revision + 1);
  });

  it('renames in place, copies with a translated name, and offers a request no children', async () => {
    await action('Login', 'rename');
    await type('http-rename-input', 'Se connecter', 'http-rename-submit');
    await action('Factures', 'duplicate');

    expect(http.callsOf('rename')).toEqual([[{ kind: 'request', id: 'Login' }, 'Se connecter']]);
    expect(http.callsOf('duplicate')).toEqual([[{ kind: 'folder', id: 'Factures' }, 'Factures (copie)']]);

    el<HTMLButtonElement>('[data-testid="http-node"][data-id="Old"] [data-testid="http-node-menu"]').click();
    await fixture.whenStable();
    expect(all('[data-testid="http-node-action"]').map((option) => option.dataset['action'])).toEqual([
      'rename',
      'duplicate',
      'delete',
    ]);
  });

  it('says what a deletion takes with it before deleting', async () => {
    const count = () => el('[data-testid="http-delete-count"]').textContent?.trim();
    http.contentsAnswer = { folders: 0, requests: 3 };
    await action('Auth', 'delete');
    expect(count()).toBe('Ses 3 requêtes partent avec.');
    el<HTMLButtonElement>('[data-testid="http-delete-cancel"]').click();

    http.contentsAnswer = { folders: 1, requests: 0 };
    await action('API', 'delete');
    expect(count()).toBe('Son dossier part avec.');

    http.contentsAnswer = { folders: 1, requests: 1 };
    await action('Factures', 'delete');
    expect(count()).toBe('1 dossier et 1 requête partent avec.');
    expect(http.callsOf('delete')).toEqual([]);

    el<HTMLButtonElement>('[data-testid="http-delete-submit"]').click();
    await fixture.whenStable();
    expect(http.callsOf('delete')).toEqual([[{ kind: 'folder', id: 'Factures' }]]);
    expect(el('[data-testid="http-delete-confirm"]')).toBeNull();
  });

  it('moves a row with Alt and the arrows, renames on F2 and asks before Delete', async () => {
    name('Logout').dispatchEvent(
      new KeyboardEvent('keydown', { key: 'ArrowUp', altKey: true, bubbles: true }),
    );
    await vi.waitFor(() => expect(http.callsOf('move')).toHaveLength(1));
    expect(http.callsOf('move')[0]).toEqual([
      { kind: 'request', id: 'Logout' },
      { collectionId: 'API', folderId: 'Auth', index: 0 },
    ]);

    name('Login').dispatchEvent(
      new KeyboardEvent('keydown', { key: 'ArrowUp', altKey: true, bubbles: true }),
    );
    await fixture.whenStable();
    expect(http.callsOf('move')).toHaveLength(1);

    name('Health').dispatchEvent(new KeyboardEvent('keydown', { key: 'F2', bubbles: true }));
    await fixture.whenStable();
    expect(el<HTMLInputElement>('[data-testid="http-rename-input"]').value).toBe('Health');

    name('Login').dispatchEvent(new KeyboardEvent('keydown', { key: 'Delete', bubbles: true }));
    await vi.waitFor(() => expect(el('[data-testid="http-delete-confirm"]')).not.toBeNull());
    expect(el('[data-testid="http-delete-count"]')).toBeNull();
    el<HTMLButtonElement>('[data-testid="http-delete-cancel"]').click();
  });

  it('moves a row dragged onto another, and the click that ends the drag opens nothing', async () => {
    const target = row('Login');
    target.getBoundingClientRect = () => ({
      top: 100,
      height: 20,
      bottom: 120,
      left: 0,
      right: 200,
      width: 200,
      x: 0,
      y: 100,
      toJSON: () => ({}),
    });
    const document = fixture.nativeElement.ownerDocument as Document;
    document.elementFromPoint = () => target;

    const health = name('Health');
    health.dispatchEvent(
      new PointerEvent('pointerdown', { button: 0, pointerId: 1, clientX: 10, clientY: 300, bubbles: true }),
    );
    health.dispatchEvent(
      new PointerEvent('pointermove', { pointerId: 1, clientX: 10, clientY: 116, bubbles: true }),
    );
    await fixture.whenStable();
    expect(target.classList).toContain('drop-after');
    expect(row('Health').classList).toContain('dragging');

    health.dispatchEvent(
      new PointerEvent('pointerup', { pointerId: 1, clientX: 10, clientY: 116, bubbles: true }),
    );
    health.click();
    await vi.waitFor(() => expect(http.callsOf('move')).toHaveLength(1));
    expect(http.callsOf('move')[0]).toEqual([
      { kind: 'request', id: 'Health' },
      { collectionId: 'API', folderId: 'Auth', index: 1 },
    ]);
    expect(TestBed.inject(HttpTabsStore).tabs()).toEqual([]);
  });

  /** « créez-en une avec ＋ » pointed at a glyph the area shows twice: the action is there itself. */
  it('offers the first collection as a button when the library holds none', async () => {
    http.tree$ = { collections: [] };
    await TestBed.inject(HttpCollectionsStore).load();
    await fixture.whenStable();

    expect(el('[data-testid="http-rail-empty"]').textContent).toContain("Aucune collection pour l'instant.");
    const create = el<HTMLButtonElement>('[data-testid="http-rail-empty-create"]');
    expect(create.textContent?.trim()).toBe('Nouvelle collection');

    create.click();
    await fixture.whenStable();
    expect(el('[data-testid="http-rail-empty"]')).toBeNull();
    expect(document.activeElement).toBe(el('[data-testid="http-create-input"]'));
    await type('http-create-input', 'Boutique', 'http-create-submit');
    expect(http.callsOf('createCollection')).toEqual([['Boutique']]);
  });

  it('opens the history from its foot, and leaves it for a request opened from the tree', async () => {
    const history = TestBed.inject(HttpHistoryStore);
    el<HTMLButtonElement>('[data-testid="http-history-open"]').click();
    await fixture.whenStable();
    expect(history.isOpen()).toBe(true);
    expect(el('[data-testid="http-history-open"]').getAttribute('aria-pressed')).toBe('true');

    name('Login').click();
    await fixture.whenStable();
    expect(history.isOpen()).toBe(false);
  });

  it('opens the cookie jar from its foot, and closes it', async () => {
    el<HTMLButtonElement>('[data-testid="http-cookies-open"]').click();
    await vi.waitFor(() =>
      expect(document.body.querySelector('[data-testid="http-cookies-dialog"]')).not.toBeNull(),
    );
    document.body.querySelector<HTMLButtonElement>('[data-testid="http-cookies-close"]')!.click();
    await fixture.whenStable();
    expect(document.body.querySelector('[data-testid="http-cookies-dialog"]')).toBeNull();
  });
});
