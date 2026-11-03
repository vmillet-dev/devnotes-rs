import { ComponentFixture, TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';
import { ExtractionRow } from '@core/model/http.model';
import { provideAppTesting } from '@testing/testing.providers';
import { ExtractDialogComponent } from './extract-dialog.component';

describe('ExtractDialogComponent', () => {
  let fixture: ComponentFixture<ExtractDialogComponent>;
  let extracted: ExtractionRow[];
  let closed: number;

  async function render(path: string, environmentName: string | null) {
    TestBed.resetTestingModule();
    TestBed.configureTestingModule({ imports: [ExtractDialogComponent], providers: [provideAppTesting()] });
    fixture = TestBed.createComponent(ExtractDialogComponent);
    fixture.componentRef.setInput('path', path);
    fixture.componentRef.setInput('requestName', 'Login');
    fixture.componentRef.setInput('environmentName', environmentName);
    extracted = [];
    closed = 0;
    fixture.componentInstance.extracted.subscribe((rule) => extracted.push(rule));
    fixture.componentInstance.closed.subscribe(() => closed++);
    fixture.autoDetectChanges();
    await fixture.whenStable();
  }

  const el = <E extends HTMLElement = HTMLElement>(testId: string) =>
    document.body.querySelector<E>(`[data-testid="${testId}"]`)!;

  beforeEach(() => render('body.access_token', 'Staging'));

  it('proposes the key in camelCase, a secret for a token, and says where it is written', () => {
    expect(el('http-extract-path').textContent).toBe('body.access_token');
    expect(el<HTMLInputElement>('http-extract-name').value).toBe('accessToken');
    expect(el<HTMLInputElement>('http-extract-secret').checked).toBe(true);
    expect(el('http-extract-into').textContent?.trim()).toBe(
      'Écrite dans l’environnement actif à chaque envoi de Login, aujourd’hui Staging.',
    );
  });

  it('extracts under the name and the secrecy chosen, and refuses a blank name', async () => {
    const name = el<HTMLInputElement>('http-extract-name');
    name.value = '  token ';
    name.dispatchEvent(new Event('input'));
    const secret = el<HTMLInputElement>('http-extract-secret');
    secret.checked = false;
    secret.dispatchEvent(new Event('change'));
    await fixture.whenStable();
    el<HTMLButtonElement>('http-extract-submit').click();
    expect(extracted).toEqual([{ path: 'body.access_token', variable: 'token', secret: false }]);

    name.value = '   ';
    name.dispatchEvent(new Event('input'));
    await fixture.whenStable();
    expect(el<HTMLButtonElement>('http-extract-submit').disabled).toBe(true);
    el('http-extract-dialog').dispatchEvent(new Event('submit'));
    expect(extracted).toHaveLength(1);

    el<HTMLButtonElement>('http-extract-cancel').click();
    expect(closed).toBe(1);
  });

  it('writes into the globals with no environment, and proposes no secret for a plain value', async () => {
    await render('body.expires_in', null);
    expect(el<HTMLInputElement>('http-extract-secret').checked).toBe(false);
    expect(el('http-extract-into').textContent).toContain('dans les globales');
  });
});
