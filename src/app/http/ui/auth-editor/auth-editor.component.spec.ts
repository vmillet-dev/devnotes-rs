import { ComponentFixture, TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';
import { DEFAULT_TRANSPORT, InheritedParts, RequestAuth } from '@core/model/http.model';
import { provideAppTesting } from '@testing/testing.providers';
import { AuthEditorComponent } from './auth-editor.component';

const FROM_API: InheritedParts = {
  auth: { kind: 'bearer', token: '{{accessToken}}' },
  authFrom: { kind: 'collection', id: 'api', name: 'API Paiements' },
  headers: [],
  transport: DEFAULT_TRANSPORT,
};

describe('AuthEditorComponent', () => {
  let fixture: ComponentFixture<AuthEditorComponent>;
  let emitted: RequestAuth[];

  async function render(auth: RequestAuth, inherited: InheritedParts | null = FROM_API, allowInherit = true) {
    TestBed.resetTestingModule();
    TestBed.configureTestingModule({ imports: [AuthEditorComponent], providers: [provideAppTesting()] });
    fixture = TestBed.createComponent(AuthEditorComponent);
    fixture.componentRef.setInput('auth', auth);
    fixture.componentRef.setInput('inherited', inherited);
    fixture.componentRef.setInput('allowInherit', allowInherit);
    emitted = [];
    fixture.componentInstance.authChange.subscribe((next) => emitted.push(next));
    fixture.autoDetectChanges();
    await fixture.whenStable();
  }

  const el = <E extends HTMLElement = HTMLElement>(selector: string): E =>
    fixture.nativeElement.querySelector(selector);
  const choose = async (testid: string, id: string) => {
    el<HTMLButtonElement>(`[data-testid="${testid}"]`).click();
    await fixture.whenStable();
    el<HTMLButtonElement>(`[data-option-id="${id}"]`).click();
    await fixture.whenStable();
  };

  beforeEach(() => render({ kind: 'inherit' }));

  it('says what an inherited auth is and where it was set, a variable shown as typed', () => {
    expect(el('[data-testid="http-auth-inherited"]').textContent?.replace(/\s+/g, ' ').trim()).toBe(
      'Bearer {{accessToken}} héritée de la collection API Paiements',
    );
  });

  it('says when nothing above sets one, and when the request has no place yet', async () => {
    await render(
      { kind: 'inherit' },
      {
        auth: { kind: 'none' },
        authFrom: null,
        headers: [],
        transport: DEFAULT_TRANSPORT,
      },
    );
    expect(el('[data-testid="http-auth-inherited"]').textContent).toContain('Rien au-dessus');

    await render({ kind: 'inherit' }, null);
    expect(el('[data-testid="http-auth-inherited"]').textContent).toContain(
      'Une fois la requête enregistrée',
    );
  });

  it('starts a chosen kind empty, and edits its fields', async () => {
    await choose('http-auth-kind', 'basic');
    expect(emitted[0]).toEqual({ kind: 'basic', username: '', password: '' });

    await render({ kind: 'basic', username: 'ada', password: '' });
    const user = el<HTMLInputElement>('[data-testid="http-auth-username"]');
    user.value = 'grace';
    user.dispatchEvent(new Event('input'));
    expect(emitted[0]).toEqual({ kind: 'basic', username: 'grace', password: '' });
  });

  it('masks the secret until asked, and masks it in a summary unless it is a variable', async () => {
    await render({ kind: 'bearer', token: 's3cret' });
    const secret = el<HTMLInputElement>('[data-testid="http-auth-secret"]');
    expect(secret.type).toBe('password');

    el<HTMLButtonElement>('[data-testid="http-auth-reveal"]').click();
    await fixture.whenStable();
    expect(el<HTMLInputElement>('[data-testid="http-auth-secret"]').type).toBe('text');

    await render(
      { kind: 'inherit' },
      { ...FROM_API, auth: { kind: 'apiKey', name: 'X-Key', value: 'abc', place: 'query' } },
    );
    expect(el('[data-testid="http-auth-inherited"]').textContent).toContain('X-Key: ••••••');
  });

  it('edits an API key and where it is sent, and offers no « Héritée » to a collection', async () => {
    await render({ kind: 'apiKey', name: '', value: '', place: 'header' }, null, false);
    const name = el<HTMLInputElement>('[data-testid="http-auth-key-name"]');
    name.value = 'X-Api-Key';
    name.dispatchEvent(new Event('input'));
    await choose('http-auth-place', 'query');

    expect(emitted).toEqual([
      { kind: 'apiKey', name: 'X-Api-Key', value: '', place: 'header' },
      { kind: 'apiKey', name: '', value: '', place: 'query' },
    ]);
    el<HTMLButtonElement>('[data-testid="http-auth-kind"]').click();
    await fixture.whenStable();
    expect(el('[data-option-id="inherit"]')).toBeNull();
  });
});
