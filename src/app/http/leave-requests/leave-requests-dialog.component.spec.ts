import { ComponentFixture, TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { HttpCollectionsStore } from '@core/services/http/http-collections.store';
import { HttpLeaveStore } from '@core/services/http/http-leave.store';
import { HttpSocketsStore } from '@core/services/http/http-sockets.store';
import { HttpTabsStore } from '@core/services/http/http-tabs.store';
import { FakeHttpRepository } from '@testing/fake-http-repository';
import { sampleTree } from '@testing/http-tree.fixture';
import { provideAppTesting } from '@testing/testing.providers';
import { LeaveRequestsDialogComponent } from './leave-requests-dialog.component';

/** One question before the page goes away with requests not saved in it, whatever takes it away. */
describe('LeaveRequestsDialogComponent', () => {
  let fixture: ComponentFixture<LeaveRequestsDialogComponent>;
  let http: FakeHttpRepository;

  const el = <E extends HTMLElement = HTMLElement>(testid: string): E =>
    document.body.querySelector(`[data-testid="${testid}"]`) as E;
  const names = () =>
    [...document.body.querySelectorAll('[data-testid="http-leave-name"]')].map((name) =>
      name.textContent?.trim(),
    );
  const leave = () => TestBed.inject(HttpLeaveStore);
  const tabs = () => TestBed.inject(HttpTabsStore);

  async function modify(id: string, renamed: string): Promise<void> {
    await tabs().open(id);
    tabs().edit(id, { name: renamed });
  }

  beforeEach(async () => {
    document.body.innerHTML = '';
    TestBed.resetTestingModule();
    http = new FakeHttpRepository();
    http.tree$ = sampleTree();
    http.seed('Login', {}, 'API', 'Auth');
    http.seed('Logout', {}, 'API', 'Auth');
    TestBed.configureTestingModule({
      imports: [LeaveRequestsDialogComponent],
      providers: [provideAppTesting({ httpRepository: http })],
    });
    await TestBed.inject(HttpCollectionsStore).load();
    fixture = TestBed.createComponent(LeaveRequestsDialogComponent);
    fixture.autoDetectChanges();
    await fixture.whenStable();
  });

  it('draws nothing while nothing is asked', () => {
    expect(el('http-leave-dialog')).toBeNull();
  });

  it('names the one request that would be lost, and what loses it', async () => {
    await modify('Login', 'Se connecter');
    void leave().confirm('quit');
    await fixture.whenStable();

    expect(el('http-leave-dialog').dataset['reason']).toBe('quit');
    expect(document.body.querySelector('#http-leave-title')?.textContent?.trim()).toBe(
      'Une requête a des modifications',
    );
    expect(document.body.querySelector('.hint')?.textContent?.trim()).toBe(
      'Elles ne sont pas enregistrées : quitter les perd.',
    );
    expect(names()).toEqual(['Se connecter']);
    expect(el('http-leave-discard').textContent?.trim()).toBe('Quitter sans enregistrer');
    expect(el('http-leave-save').textContent?.trim()).toBe('Tout enregistrer');
    expect(el('http-leave-connected')).toBeNull();
  });

  it('counts several, a request never saved among them, and says it of a library switch', async () => {
    await modify('Login', 'Se connecter');
    await modify('Logout', 'Sortir');
    tabs().newRequest();
    tabs().edit(tabs().activeKey()!, { name: 'Brouillon' });
    void leave().confirm('switch');
    await fixture.whenStable();

    expect(document.body.querySelector('#http-leave-title')?.textContent?.trim()).toBe(
      '3 requêtes ont des modifications',
    );
    expect(names()).toEqual(['Se connecter', 'Sortir', 'Brouillon']);
    expect(el('http-leave-discard').textContent?.trim()).toBe('Changer sans enregistrer');
    expect(document.body.querySelector('.hint')?.textContent?.trim()).toBe(
      'Elles ne sont pas enregistrées : changer de bibliothèque les perd.',
    );
  });

  it('says a connected socket goes too, in the same question', async () => {
    http.seed('Live', { kind: 'websocket' }, 'Flux');
    await tabs().open('Live');
    await TestBed.inject(HttpSocketsStore).connect(tabs().active()!);
    await modify('Login', 'Se connecter');
    void leave().confirm('restore');
    await fixture.whenStable();

    expect(el('http-leave-connected').textContent?.trim()).toBe('Un WebSocket connecté sera fermé.');
    expect(el('http-leave-discard').textContent?.trim()).toBe('Restaurer sans enregistrer');
  });

  it('answers from its three buttons: stay, go without saving, save everything then go', async () => {
    await modify('Login', 'Se connecter');

    const staying = leave().confirm('update');
    await fixture.whenStable();
    expect(el('http-leave-discard').textContent?.trim()).toBe('Installer sans enregistrer');
    el<HTMLButtonElement>('http-leave-cancel').click();
    expect(await staying).toBe(false);
    await fixture.whenStable();
    expect(el('http-leave-dialog')).toBeNull();

    const discarding = leave().confirm('quit');
    await fixture.whenStable();
    el<HTMLButtonElement>('http-leave-discard').click();
    expect(await discarding).toBe(true);
    expect(http.callsOf('saveRequest')).toEqual([]);

    const saving = leave().confirm('quit');
    await fixture.whenStable();
    el<HTMLButtonElement>('http-leave-save').click();
    expect(await saving).toBe(true);
    expect(http.callsOf('saveRequest').map((call) => call[0])).toEqual(['Login']);
  });

  /** A request never saved has no collection: the save dialog is asked for each, above the question. */
  it('gives way to the save dialog for a request never saved, and comes back when it is cancelled', async () => {
    tabs().newRequest();
    tabs().edit(tabs().activeKey()!, { name: 'Brouillon' });
    let gone: boolean | null = null;
    void leave()
      .confirm('quit')
      .then((go) => (gone = go));
    await fixture.whenStable();

    el<HTMLButtonElement>('http-leave-save').click();
    await vi.waitFor(() => expect(el('http-save-dialog')).not.toBeNull());
    expect(el('http-leave-dialog')).toBeNull();
    expect(el<HTMLInputElement>('http-save-name').value).toBe('Brouillon');

    el<HTMLButtonElement>('http-save-cancel').click();
    await vi.waitFor(() => expect(el('http-leave-dialog')).not.toBeNull());
    expect(gone).toBeNull();

    el<HTMLButtonElement>('http-leave-save').click();
    await vi.waitFor(() => expect(el('http-save-dialog')).not.toBeNull());
    el<HTMLButtonElement>('http-save-submit').click();
    await vi.waitFor(() => expect(gone).toBe(true));
    expect(http.callsOf('createRequest').at(-1)?.[0]).toMatchObject({
      name: 'Brouillon',
      collectionId: 'API',
    });
  });
});
