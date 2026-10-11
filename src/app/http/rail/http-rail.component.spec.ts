import { ComponentFixture, TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { HttpCollectionsStore } from '@core/services/http/http-collections.store';
import { HttpHistoryStore } from '@core/services/http/http-history.store';
import { HttpTabsStore } from '@core/services/http/http-tabs.store';
import { RAIL_WIDTH } from '@core/services/settings/app-settings.model';
import { SettingsStore } from '@core/services/settings/settings.store';
import { FakeHttpRepository } from '@testing/fake-http-repository';
import { collection, folder, request, sampleTree } from '@testing/http-tree.fixture';
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
    // The three kinds of request are one entry of the menu, and a submenu beside it.
    if (/^new(Http|Graphql|Websocket)$/.test(which)) {
      el<HTMLButtonElement>('[data-testid="http-node-new"]').click();
      await fixture.whenStable();
    }
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

  /**
   * « Nouvelle requête » made an HTTP `GET`, whatever was meant: its kind is chosen first.
   * Listed flat, the three kinds made eight entries of a menu opened to rename something:
   * they are one entry, and a submenu beside it.
   */
  it('offers the three kinds of request behind one entry of a row’s menu, and creates the one chosen', async () => {
    const actions = () => all('[data-testid="http-node-action"]').map((entry) => entry.dataset['action']);
    el<HTMLButtonElement>('[data-testid="http-node"][data-id="Auth"] [data-testid="http-node-menu"]').click();
    await fixture.whenStable();
    const parent = el<HTMLButtonElement>('[data-testid="http-node-new"]');
    expect(parent.textContent).toContain('Nouvelle requête');
    expect(parent.getAttribute('aria-expanded')).toBe('false');
    expect(actions()).toEqual(['newFolder', 'settings', 'rename', 'duplicate', 'delete']);

    // The right arrow opens it as a click does, on its first kind; the left one comes back.
    parent.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowRight', bubbles: true }));
    await fixture.whenStable();
    expect(parent.getAttribute('aria-expanded')).toBe('true');
    const kinds = () =>
      all('[data-testid="http-node-new-menu"] [data-testid="http-node-action"]').map((entry) => [
        entry.dataset['action'],
        entry.textContent?.trim(),
      ]);
    expect(kinds()).toEqual([
      ['newHttp', 'Requête HTTP'],
      ['newGraphql', 'Requête GraphQL'],
      ['newWebsocket', 'WebSocket'],
    ]);
    await vi.waitFor(() => expect(document.activeElement?.getAttribute('data-action')).toBe('newHttp'));
    document.activeElement!.dispatchEvent(new KeyboardEvent('keydown', { key: 'ArrowLeft', bubbles: true }));
    await fixture.whenStable();
    expect(el('[data-testid="http-node-new-menu"]')).toBeNull();
    expect(document.activeElement).toBe(parent);

    // Escape folds the submenu first, the menu next.
    parent.click();
    await fixture.whenStable();
    await vi.waitFor(() => expect(document.activeElement?.getAttribute('data-action')).toBe('newHttp'));
    document.activeElement!.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
    await fixture.whenStable();
    expect(el('[data-testid="http-node-new-menu"]')).toBeNull();
    expect(el('[data-testid="http-node-menu-panel"]')).not.toBeNull();

    parent.click();
    await fixture.whenStable();
    el<HTMLButtonElement>('[data-testid="http-node-action"][data-action="newGraphql"]').click();
    await fixture.whenStable();
    expect(el('[data-testid="http-node-menu-panel"]')).toBeNull();
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

  /**
   * Deleting a folder closed the tabs of its requests, a modified one like any other, with
   * what was typed in it. Rust counts what is deleted; only the front knows what is unsaved.
   */
  describe('a deletion that takes open tabs with unsaved changes', () => {
    const warning = () => el('[data-testid="http-delete-unsaved"]');
    const names = () =>
      all('[data-testid="http-delete-unsaved-name"]').map((entry) => entry.textContent?.trim());
    const modify = async (id: string, renamed: string) => {
      http.seed(id);
      const tabs = TestBed.inject(HttpTabsStore);
      await tabs.open(id);
      tabs.edit(id, { name: renamed });
      await fixture.whenStable();
    };

    it('says so for a folder, with the names, and still asks one question', async () => {
      await modify('Login', 'Se connecter');
      http.contentsAnswer = { folders: 0, requests: 2 };
      await action('Auth', 'delete');

      expect(warning().textContent).toContain(
        'Une requête ouverte a des modifications non enregistrées : elles seront perdues.',
      );
      expect(names()).toEqual(['Se connecter']);
      expect(all('[data-testid="http-delete-confirm"]')).toHaveLength(1);
      expect(el('[data-testid="http-delete-count"]').textContent?.trim()).toBe(
        'Ses 2 requêtes partent avec.',
      );

      // Cancelled: the tab and what was typed in it are as they were.
      el<HTMLButtonElement>('[data-testid="http-delete-cancel"]').click();
      await fixture.whenStable();
      expect(http.callsOf('delete')).toEqual([]);
      expect(TestBed.inject(HttpTabsStore).active()?.draft.name).toBe('Se connecter');
    });

    it('counts a tab under a sub-folder, and every one a collection holds', async () => {
      await modify('Old', 'Vieille');
      await action('Factures', 'delete');
      expect(names()).toEqual(['Vieille']);
      el<HTMLButtonElement>('[data-testid="http-delete-cancel"]').click();
      await fixture.whenStable();

      await modify('Login', 'Se connecter');
      await action('API', 'delete');
      expect(warning().textContent).toContain('2 requêtes ouvertes ont des modifications non enregistrées');
      expect(names()).toEqual(['Vieille', 'Se connecter']);
    });

    it('says it of the request itself when its own row is deleted', async () => {
      await modify('Login', 'Se connecter');
      await action('Login', 'delete');

      expect(warning().textContent?.trim()).toBe(
        'Elle est ouverte avec des modifications non enregistrées : elles seront perdues.',
      );
      expect(names()).toEqual([]);
    });

    it('says nothing for a tab that is open and not modified, nor for one elsewhere', async () => {
      await TestBed.inject(HttpTabsStore).open('Login');
      await fixture.whenStable();
      await action('Auth', 'delete');
      expect(warning()).toBeNull();
      el<HTMLButtonElement>('[data-testid="http-delete-cancel"]').click();
      await fixture.whenStable();

      await modify('Login', 'Se connecter');
      await action('Factures', 'delete');
      expect(warning()).toBeNull();
    });

    it('reads the tabs as the question stands: one saved meanwhile is no longer a loss', async () => {
      await modify('Login', 'Se connecter');
      await action('Auth', 'delete');
      expect(names()).toEqual(['Se connecter']);

      await TestBed.inject(HttpTabsStore).save('Login');
      await fixture.whenStable();
      expect(warning()).toBeNull();
    });
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

  /**
   * With the pointer the drop target is shown; the keys showed nothing, and at one indent a
   * row above a folder looks like a row inside it. The fake keeps no tree: the one Rust would
   * answer after the move is set before the key.
   */
  describe('a move made with the keys', () => {
    const alt = (id: string, key: string) =>
      name(id).dispatchEvent(new KeyboardEvent('keydown', { key, altKey: true, bubbles: true }));
    const marks = () =>
      Object.fromEntries(
        all('[data-testid="http-node"][data-moved]').map((node) => [
          node.dataset['id'],
          node.dataset['moved'],
        ]),
      );
    const said = () => el('[data-testid="http-rail-moved"]').textContent?.trim();
    /** Health in Factures, after Archives: what `Alt+→` on Health asks for. */
    const healthInFactures = () => ({
      collections: [
        collection('API', [
          folder('Auth', 'API', [request('Login', 'API', 'Auth'), request('Logout', 'API', 'Auth')]),
          folder('Factures', 'API', [
            folder('Archives', 'API', [request('Old', 'API', 'Archives')], 'Factures'),
            request('Health', 'API', 'Factures', 'POST'),
          ]),
        ]),
      ],
    });

    it('marks the row and the folder that now holds it, and says where it went', async () => {
      expect(el('[data-testid="http-rail-moved"]').getAttribute('role')).toBe('status');
      expect(said()).toBe('');

      http.tree$ = healthInFactures();
      alt('Health', 'ArrowRight');

      await vi.waitFor(() => expect(marks()).toEqual({ Health: 'row', Factures: 'parent' }));
      expect(said()).toBe('Health est maintenant dans Factures.');
      expect(el('[data-testid="http-rail-moved"]').dataset['said']).toBe('into');
      expect(row('Health').style.getPropertyValue('--depth')).toBe('2');
    });

    it('unfolds a folded folder that receives the row, instead of letting it vanish', async () => {
      el<HTMLButtonElement>(
        '[data-testid="http-node"][data-id="Factures"] [data-testid="http-node-twisty"]',
      ).click();
      await fixture.whenStable();
      expect(row('Archives')).toBeNull();

      http.tree$ = healthInFactures();
      alt('Health', 'ArrowRight');

      await vi.waitFor(() => expect(marks()).toEqual({ Health: 'row', Factures: 'parent' }));
      expect(row('Archives')).not.toBeNull();
    });

    it('says the root of a collection, the row passed going up or down, and marks a collection alone', async () => {
      http.tree$ = {
        collections: [
          collection('API', [
            folder('Auth', 'API', [request('Logout', 'API', 'Auth')]),
            request('Login', 'API'),
          ]),
        ],
      };
      alt('Login', 'ArrowLeft');
      await vi.waitFor(() => expect(said()).toBe('Login est maintenant à la racine de API.'));
      expect(marks()).toEqual({ Login: 'row', API: 'parent' });

      http.tree$ = sampleTree();
      await TestBed.inject(HttpCollectionsStore).load();
      http.tree$ = {
        collections: [
          collection('API', [
            folder('Auth', 'API', [request('Logout', 'API', 'Auth'), request('Login', 'API', 'Auth')]),
          ]),
        ],
      };
      alt('Logout', 'ArrowUp');
      await vi.waitFor(() => expect(said()).toBe('Logout est maintenant avant Login.'));
      expect(marks()).toEqual({ Logout: 'row', Auth: 'parent' });

      http.tree$ = sampleTree();
      await TestBed.inject(HttpCollectionsStore).load();
      http.tree$ = { collections: [collection('Catalogue'), collection('API'), collection('Flux')] };
      alt('API', 'ArrowDown');
      await vi.waitFor(() => expect(said()).toBe('API est maintenant après Catalogue.'));
      expect(marks()).toEqual({ API: 'row' });
    });

    it('says why a move cannot be made instead of doing nothing, each time it is asked', async () => {
      const sentence = () => el('[data-testid="http-rail-moved"]').firstChild;

      alt('Login', 'ArrowUp');
      await fixture.whenStable();
      expect(said()).toBe('Login est déjà en premier.');
      // The same sentence twice is two things said: a new node, or no reader hears the second.
      const first = sentence();
      alt('Login', 'ArrowUp');
      await fixture.whenStable();
      expect(said()).toBe('Login est déjà en premier.');
      expect(sentence()).not.toBe(first);

      alt('Health', 'ArrowDown');
      await fixture.whenStable();
      expect(said()).toBe('Health est déjà en dernier.');
      alt('Auth', 'ArrowRight');
      await fixture.whenStable();
      expect(said()).toBe('Aucun dossier juste au-dessus de Auth où entrer.');
      alt('Health', 'ArrowLeft');
      await fixture.whenStable();
      expect(said()).toBe('Health est déjà à la racine de sa collection.');
      alt('API', 'ArrowRight');
      await fixture.whenStable();
      expect(said()).toBe('Une collection ne se range pas dans une autre.');

      expect(http.callsOf('move')).toEqual([]);
      expect(http.callsOf('reorderCollection')).toEqual([]);
      expect(marks()).toEqual({});
    });

    it('keeps the mark a moment, and starts its fade again for a row moved twice', async () => {
      vi.useFakeTimers({ toFake: ['setTimeout', 'clearTimeout'] });
      try {
        const beat = () => el('.tree').dataset['beat'];
        const before = beat();
        http.tree$ = healthInFactures();
        alt('Health', 'ArrowRight');
        await vi.waitFor(() => expect(marks()).toEqual({ Health: 'row', Factures: 'parent' }));
        expect(beat()).not.toBe(before);

        // Up past Archives: the row keeps its mark, and only a change of beat fades it again.
        const once = beat();
        http.tree$ = {
          collections: [
            collection('API', [
              folder('Factures', 'API', [
                request('Health', 'API', 'Factures', 'POST'),
                folder('Archives', 'API', [], 'Factures'),
              ]),
            ]),
          ],
        };
        alt('Health', 'ArrowUp');
        await vi.waitFor(() => expect(said()).toBe('Health est maintenant avant Archives.'));
        expect(marks()).toEqual({ Health: 'row', Factures: 'parent' });
        expect(beat()).not.toBe(once);

        vi.advanceTimersByTime(2000);
        await fixture.whenStable();
        expect(marks()).toEqual({});
        // What was said stays: a live region is heard when it changes, not while it lasts.
        expect(said()).toBe('Health est maintenant avant Archives.');
      } finally {
        vi.useRealTimers();
      }
    });
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
