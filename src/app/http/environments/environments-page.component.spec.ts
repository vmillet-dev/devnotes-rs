import { ComponentFixture, TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { HttpEnvironmentsStore } from '@core/services/http/http-environments.store';
import {
  FakeHttpEnvironmentsRepository,
  environment,
  variable,
} from '@testing/fake-http-environments-repository';
import { provideAppTesting } from '@testing/testing.providers';
import { FakeHttpRepository } from '@testing/fake-http-repository';
import { HttpScriptFocusStore } from '@core/services/http/http-script-focus.store';
import { HttpTabsStore } from '@core/services/http/http-tabs.store';
import { HttpWorkspaceStore } from '@core/services/http/http-workspace.store';
import { EnvironmentsPageComponent } from './environments-page.component';

describe('EnvironmentsPageComponent, a variable a request fills', () => {
  it('opens the request that fills it on its scripts after the answer', async () => {
    TestBed.resetTestingModule();
    const http = new FakeHttpRepository();
    http.seed('login', { name: 'Login' });
    const environments = new FakeHttpEnvironmentsRepository({
      environments: [
        environment('Local', {
          variables: [variable('accessToken', 'eyJ', { filledBy: { requestId: 'login', name: 'Login' } })],
        }),
      ],
    });
    TestBed.configureTestingModule({
      imports: [EnvironmentsPageComponent],
      providers: [provideAppTesting({ httpEnvironmentsRepository: environments, httpRepository: http })],
    });
    const fixture = TestBed.createComponent(EnvironmentsPageComponent);
    fixture.autoDetectChanges();
    await vi.waitFor(() => expect(TestBed.inject(HttpEnvironmentsStore).loaded()).toBe(true));
    TestBed.inject(HttpEnvironmentsStore).select({ kind: 'environment', id: 'env-local' });
    await fixture.whenStable();
    TestBed.inject(HttpWorkspaceStore).show('environments');

    (fixture.nativeElement as HTMLElement)
      .querySelector<HTMLButtonElement>('[data-testid="http-variable-filled"]')!
      .click();

    await vi.waitFor(() => expect(TestBed.inject(HttpTabsStore).active()?.key).toBe('login'));
    expect(TestBed.inject(HttpWorkspaceStore).page()).toBe('requests');
    expect(TestBed.inject(HttpScriptFocusStore).request()).toEqual({
      key: 'login',
      focus: { phase: 'after', line: null },
    });
  });
});

describe('EnvironmentsPageComponent', () => {
  let fixture: ComponentFixture<EnvironmentsPageComponent>;
  let environments: FakeHttpEnvironmentsRepository;

  beforeEach(async () => {
    TestBed.resetTestingModule();
    environments = new FakeHttpEnvironmentsRepository({
      globals: [variable('tenant', 'acme')],
      environments: [
        environment('Local', { colour: 'green', variables: [variable('baseUrl', 'http://localhost')] }),
        environment('Production', {
          colour: 'red',
          protected: true,
          variables: [variable('baseUrl', 'https://api'), variable('token', 't', { kind: 'secret' })],
        }),
      ],
    });
    TestBed.configureTestingModule({
      imports: [EnvironmentsPageComponent],
      providers: [provideAppTesting({ httpEnvironmentsRepository: environments })],
    });
    fixture = TestBed.createComponent(EnvironmentsPageComponent);
    fixture.autoDetectChanges();
    await vi.waitFor(() => expect(store().loaded()).toBe(true));
    await fixture.whenStable();
  });

  const store = () => TestBed.inject(HttpEnvironmentsStore);
  const all = <E extends HTMLElement = HTMLElement>(selector: string): E[] => [
    ...fixture.nativeElement.querySelectorAll(selector),
  ];
  const one = <E extends HTMLElement = HTMLElement>(selector: string): E =>
    fixture.nativeElement.querySelector(selector) as E;
  const entry = (id: string) =>
    all('[data-testid="http-environment-entry"]').find((row) => row.dataset['id'] === id);

  it('lists the globals first, then each environment with what it holds', () => {
    const entries = all('[data-testid="http-environment-entry"]');
    expect(entries.map((row) => row.dataset['id'])).toEqual(['globals', 'env-local', 'env-production']);
    expect(entries[2]?.textContent).toContain('protégé');
    expect(entries[2]?.textContent).toContain('2 variables');
    expect(entries[2]?.textContent).toContain('1 secrète');
  });

  it('opens on the globals, and shows an environment chosen', async () => {
    expect(all<HTMLInputElement>('[data-testid="http-variable-name"]')[0]?.value).toBe('tenant');

    entry('env-production')?.click();
    await fixture.whenStable();

    expect(one<HTMLInputElement>('[data-testid="http-environment-name"]').value).toBe('Production');
    expect(one<HTMLInputElement>('[data-testid="http-environment-protected"]').checked).toBe(true);
    expect(one('[data-testid="http-environment-protected-note"]')).not.toBeNull();
    expect(all<HTMLInputElement>('[data-testid="http-variable-value"]')[1]?.type).toBe('password');
  });

  it('creates an environment by its name and shows it', async () => {
    one('[data-testid="http-environment-create-open"]').click();
    await fixture.whenStable();
    const field = one<HTMLInputElement>('[data-testid="http-environment-create-input"]');
    field.value = 'Staging';
    one('[data-testid="http-environment-create-submit"]').click();

    await vi.waitFor(() => expect(environments.callsTo('create')).toHaveLength(1));
    await fixture.whenStable();
    expect(one<HTMLInputElement>('[data-testid="http-environment-name"]').value).toBe('Staging');
  });

  it('protects, makes active and duplicates the one shown', async () => {
    entry('env-local')?.click();
    await fixture.whenStable();

    one<HTMLInputElement>('[data-testid="http-environment-protected"]').click();
    await vi.waitFor(() => expect(store().shown()?.protected).toBe(true));

    one('[data-testid="http-environment-use"]').click();
    await fixture.whenStable();
    expect(store().active()?.name).toBe('Local');
    expect(one('[data-testid="http-environment-active"]')).not.toBeNull();

    one('[data-testid="http-environment-duplicate"]').click();
    await vi.waitFor(() => expect(environments.callsTo('duplicate')).toHaveLength(1));
    expect(environments.callsTo('duplicate')[0]?.args).toEqual(['env-local', 'Local (copie)']);
  });

  it('deletes only once asked twice', async () => {
    entry('env-local')?.click();
    await fixture.whenStable();

    one('[data-testid="http-environment-delete"]').click();
    await fixture.whenStable();
    expect(environments.callsTo('delete')).toEqual([]);
    expect(one('[data-testid="http-environment-delete-question"]').textContent).toContain('Local');

    one('[data-testid="http-environment-delete-confirm"]').click();
    await vi.waitFor(() => expect(environments.callsTo('delete')).toHaveLength(1));
    await fixture.whenStable();
    expect(all('[data-testid="http-environment-entry"]').map((row) => row.dataset['id'])).toEqual([
      'globals',
      'env-production',
    ]);
  });

  it('renames on leaving the field, and writes nothing for the same name', async () => {
    entry('env-local')?.click();
    await fixture.whenStable();
    const name = one<HTMLInputElement>('[data-testid="http-environment-name"]');

    name.value = 'Local';
    name.dispatchEvent(new Event('change'));
    expect(environments.callsTo('save')).toEqual([]);

    name.value = 'Dev';
    name.dispatchEvent(new Event('change'));
    await vi.waitFor(() => expect(store().shown()?.name).toBe('Dev'));
  });

  it('recolours from its swatches', async () => {
    entry('env-local')?.click();
    await fixture.whenStable();

    all('[data-testid="http-environment-colour"]')
      .find((swatch) => swatch.dataset['colour'] === 'purple')
      ?.click();

    await vi.waitFor(() => expect(store().shown()?.colour).toBe('purple'));
  });
});
