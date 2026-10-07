import { ComponentFixture, TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it, vi } from 'vitest';
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
    };
    http.inheritedAnswer = {
      auth: { kind: 'bearer', token: '{{accessToken}}' },
      authFrom: { kind: 'collection', id: 'api', name: 'API' },
      headers: [],
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
    await fixture.whenStable();

    el<HTMLButtonElement>('[data-testid="http-settings-save"]').click();
    await vi.waitFor(() => expect(closed).toBe(1));
    expect(http.callsOf('saveSettings')).toEqual([
      [
        { kind: 'folder', id: 'factures' },
        {
          auth: { kind: 'inherit' },
          headers: [{ enabled: true, key: 'Accept', value: 'application/json', description: '' }],
        },
      ],
    ]);
  });

  it('asks a collection nothing about what is above it, and closes on Annuler', async () => {
    await render({ ...FOLDER, kind: 'collection', id: 'api', name: 'API', collectionId: 'api', depth: 0 });
    expect(http.callsOf('inherited')).toEqual([]);

    el<HTMLButtonElement>('[data-testid="http-settings-cancel"]').click();
    expect(closed).toBe(1);
  });
});
