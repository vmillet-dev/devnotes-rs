import { ComponentFixture, TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { IpcError } from '@core/ipc/ipc.error';
import { DEFAULT_TRANSPORT } from '@core/model/http.model';
import { ErrorNotifier } from '@core/services/errors/error-notifier.service';
import { RailRow } from '@core/services/http/http-tree';
import { FakeHttpRepository } from '@testing/fake-http-repository';
import { provideAppTesting } from '@testing/testing.providers';
import { ContainerSettingsDialogComponent } from './container-settings-dialog.component';

const FOLDER: RailRow = {
  kind: 'folder',
  id: 'factures',
  name: 'Factures',
  depth: 1,
  collectionId: 'api',
  folderId: null,
  expanded: true,
  badge: null,
  requestKind: null,
};

describe('ContainerSettingsDialogComponent', () => {
  let fixture: ComponentFixture<ContainerSettingsDialogComponent>;
  let http: FakeHttpRepository;
  let closed: number;

  async function render(row: RailRow) {
    TestBed.resetTestingModule();
    http = new FakeHttpRepository();
    http.settingsAnswer = {
      auth: { kind: 'inherit' },
      headers: [{ enabled: true, key: 'Accept', value: 'text/csv', description: '' }],
      transport: {},
      variables: [{ enabled: true, name: 'tenant', value: 'acme', kind: 'text' }],
    };
    http.inheritedAnswer = {
      auth: { kind: 'bearer', token: '{{accessToken}}' },
      authFrom: { kind: 'collection', id: 'api', name: 'API' },
      headers: [],
      transport: DEFAULT_TRANSPORT,
    };
    TestBed.configureTestingModule({
      imports: [ContainerSettingsDialogComponent],
      providers: [provideAppTesting({ httpRepository: http })],
    });
    fixture = TestBed.createComponent(ContainerSettingsDialogComponent);
    fixture.componentRef.setInput('row', row);
    closed = 0;
    fixture.componentInstance.closed.subscribe(() => closed++);
    fixture.autoDetectChanges();
    await vi.waitFor(() => expect(http.callsOf('settings')).toHaveLength(1));
    await fixture.whenStable();
  }

  const el = <E extends HTMLElement = HTMLElement>(selector: string): E =>
    document.body.querySelector(selector) as E;

  beforeEach(() => render(FOLDER));

  it('reads a folder’s settings, and what it would inherit from above', async () => {
    await vi.waitFor(() => expect(el('[data-testid="http-auth-inherited"]')?.textContent).toContain('API'));
    expect(http.callsOf('inherited')).toEqual([['api', null]]);
    expect(el<HTMLInputElement>('[data-testid="http-kv-key"]').value).toBe('Accept');
  });

  it('saves the headers and the auth as edited, then closes', async () => {
    await vi.waitFor(() => expect(el('[data-testid="http-kv-value"]')).not.toBeNull());
    const value = el<HTMLInputElement>('[data-testid="http-kv-value"]');
    value.value = 'application/json';
    value.dispatchEvent(new Event('input'));
    const timeout = el<HTMLInputElement>('[data-testid="http-transport-timeoutMs"]');
    timeout.value = '8000';
    timeout.dispatchEvent(new Event('change'));
    await fixture.whenStable();

    el<HTMLButtonElement>('[data-testid="http-settings-save"]').click();
    await vi.waitFor(() => expect(closed).toBe(1));
    expect(http.callsOf('saveSettings')).toEqual([
      [
        { kind: 'folder', id: 'factures' },
        {
          auth: { kind: 'inherit' },
          headers: [{ enabled: true, key: 'Accept', value: 'application/json', description: '' }],
          transport: { timeoutMs: 8000 },
          variables: [{ enabled: true, name: 'tenant', value: 'acme', kind: 'text' }],
        },
      ],
    ]);
  });

  /**
   * A header stored with its value in its name failed every request under the folder, on
   * « this URL cannot be sent ». Refused here, where it is typed, and said for what it is.
   */
  it('marks a header that cannot be sent, and stays open when Rust refuses to keep it', async () => {
    await vi.waitFor(() => expect(el('[data-testid="http-kv-key"]')).not.toBeNull());
    http.headerFaults = ['name'];
    const name = el<HTMLInputElement>('[data-testid="http-kv-key"]');
    name.value = 'X Recette';
    name.dispatchEvent(new Event('input'));
    await vi.waitFor(() => expect(el('[data-testid="http-kv-row"]').dataset['fault']).toBe('name'));
    expect(el('[data-testid="http-kv-fault"]').textContent).toContain(
      'Un nom d’en-tête ne contient ni espace',
    );
    await fixture.whenStable();

    http.failNext = new IpcError('save_http_settings', {
      code: 'httpHeaderName',
      params: { header: 'X Recette', source: 'request' },
      detail: 'The header X Recette cannot be sent',
    });
    el<HTMLButtonElement>('[data-testid="http-settings-save"]').click();
    await vi.waitFor(() =>
      expect(TestBed.inject(ErrorNotifier).notice()?.ref).toEqual({
        key: 'errors.httpHeaderName.request',
        params: { header: 'X Recette', source: 'request' },
      }),
    );
    expect(closed).toBe(0);
    expect(el('[data-testid="http-settings-dialog"]')).not.toBeNull();
  });

  it('asks a collection nothing about what is above it, and closes on Annuler', async () => {
    await render({ ...FOLDER, kind: 'collection', id: 'api', name: 'API', collectionId: 'api', depth: 0 });
    expect(http.callsOf('inherited')).toEqual([]);
    await vi.waitFor(() =>
      expect(el('[data-testid="http-transport-from"]')?.textContent).toContain('Par défaut'),
    );

    el<HTMLButtonElement>('[data-testid="http-settings-cancel"]').click();
    expect(closed).toBe(1);
  });
});
