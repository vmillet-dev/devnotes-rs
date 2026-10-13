import { ComponentFixture, TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { CodeOptions } from '@core/model/http-code.model';
import { EMPTY_PARTS } from '@core/model/http.model';
import { RequestTab } from '@core/services/http/http-tabs.store';
import { FakeClipboard } from '@testing/fake-clipboard';
import { FakeHttpRepository } from '@testing/fake-http-repository';
import { provideAppTesting } from '@testing/testing.providers';
import { CodeDialogComponent } from './code-dialog.component';

const TAB: RequestTab = {
  key: 'Login',
  requestId: 'Login',
  place: { collectionId: 'API', folderId: null },
  saved: { name: 'Login', kind: 'http', method: 'POST', parts: { ...EMPTY_PARTS, url: '{{baseUrl}}/login' } },
  draft: { name: 'Login', kind: 'http', method: 'POST', parts: { ...EMPTY_PARTS, url: '{{baseUrl}}/login' } },
};

describe('CodeDialogComponent', () => {
  let fixture: ComponentFixture<CodeDialogComponent>;
  let http: FakeHttpRepository;
  let clipboard: FakeClipboard;

  beforeEach(async () => {
    TestBed.resetTestingModule();
    http = new FakeHttpRepository();
    clipboard = new FakeClipboard();
    http.codeAnswer = {
      code: "--url 'https://api/login' --header 'Authorization: Bearer {{token}}'",
      language: 'sh',
      secrets: ['token'],
      unresolved: [],
      notice: null,
    };
    TestBed.configureTestingModule({
      imports: [CodeDialogComponent],
      providers: [provideAppTesting({ httpRepository: http, clipboard })],
    });
    fixture = TestBed.createComponent(CodeDialogComponent);
    fixture.componentRef.setInput('tab', TAB);
    fixture.autoDetectChanges();
    await vi.waitFor(() => expect(http.callsOf('generateCode')).toHaveLength(1));
    await fixture.whenStable();
    el('[data-testid="http-code-target"][data-target="curl"]').click();
    await fixture.whenStable();
  });

  const el = <E extends HTMLElement = HTMLElement>(selector: string): E =>
    document.querySelector(selector) as E;
  const all = (selector: string): HTMLElement[] => [...document.querySelectorAll<HTMLElement>(selector)];
  const lastOptions = () => http.callsOf('generateCode').at(-1)?.[1] as CodeOptions | undefined;

  it('lists the targets by group and writes the one chosen, resolved, on several lines', async () => {
    expect(all('[data-testid="http-code-target"]').map((target) => target.dataset['target'])).toEqual([
      'curl',
      'httpie',
      'wget',
      'powershell',
      'fetch',
      'axios',
      'undici',
      'angular',
      'requests',
      'httpx',
      'go',
      'reqwest',
      'java',
      'okhttp',
      'csharp',
      'guzzle',
      'ruby',
      'swift',
      'dart',
      'elixir',
    ]);
    expect(lastOptions()).toEqual({
      resolveVariables: true,
      includeSecrets: false,
      multiline: true,
      followRedirects: false,
    });

    el('[data-testid="http-code-target"][data-target="powershell"]').click();
    await vi.waitFor(() => expect(http.callsOf('generateCode').at(-1)?.[0]).toBe('powershell'));
    await fixture.whenStable();
    expect(el('[data-testid="http-code-output"]').textContent).toContain('powershell --url');
  });

  it('cuts a command line, and lays a program out whatever the switch says', async () => {
    expect(el<HTMLInputElement>('[data-testid="http-code-multiline"]').disabled).toBe(false);

    el('[data-testid="http-code-target"][data-target="angular"]').click();
    await vi.waitFor(() => expect(http.callsOf('generateCode').at(-1)?.[0]).toBe('angular'));
    await fixture.whenStable();

    expect(el<HTMLInputElement>('[data-testid="http-code-multiline"]').disabled).toBe(true);
    expect(el('[data-testid="http-code-target"][data-target="angular"]').textContent).toContain(
      'Angular HttpClient',
    );
  });

  it('files the other languages under their own group', () => {
    const groups = all('.group').map((group) => group.textContent?.trim());
    expect(groups).toEqual(['Shell', 'JavaScript', 'Autres langages']);
  });

  it('filters the targets by their name or their library', async () => {
    const filter = el<HTMLInputElement>('[data-testid="http-code-filter"]');
    filter.value = 'invoke';
    filter.dispatchEvent(new Event('input'));
    await fixture.whenStable();

    expect(all('[data-testid="http-code-target"]').map((target) => target.dataset['target'])).toEqual([
      'powershell',
    ]);
  });

  it('says which secret was left as a variable, by a parameter and never inside the sentence', () => {
    const note = el('[data-testid="http-code-note"]');
    expect(note.dataset['note']).toBe('secrets');
    expect(note.textContent).toContain('1 secret laissé en variable');
    expect(note.textContent).toContain('{{token}}');
  });

  it('asks every time before writing the secrets in clear', async () => {
    const secrets = el<HTMLInputElement>('[data-testid="http-code-secrets"]');
    secrets.click();
    await fixture.whenStable();
    expect(secrets.checked).toBe(false);
    expect(el('[data-testid="http-code-secrets-ask"]')).not.toBeNull();
    expect(lastOptions()?.includeSecrets).toBe(false);

    el('[data-testid="http-code-secrets-yes"]').click();
    await vi.waitFor(() => expect(lastOptions()?.includeSecrets).toBe(true));
    await fixture.whenStable();
    expect(secrets.checked).toBe(true);
    expect(el('[data-testid="http-code-note"]').dataset['note']).toBe('clear');

    secrets.click();
    await vi.waitFor(() => expect(lastOptions()?.includeSecrets).toBe(false));
    secrets.click();
    await fixture.whenStable();
    expect(el('[data-testid="http-code-secrets-ask"]')).not.toBeNull();
  });

  it('turns the other switches each on its own', async () => {
    el('[data-testid="http-code-multiline"]').click();
    el('[data-testid="http-code-follow"]').click();
    el('[data-testid="http-code-resolve"]').click();
    await vi.waitFor(() =>
      expect(lastOptions()).toEqual({
        resolveVariables: false,
        includeSecrets: false,
        multiline: false,
        followRedirects: true,
      }),
    );
  });

  it('says what nothing sets and what a target cannot write', async () => {
    http.codeAnswer = { ...http.codeAnswer, unresolved: ['tenant'], notice: 'multipartUnsupported' };
    el('[data-testid="http-code-target"][data-target="wget"]').click();
    await vi.waitFor(() => expect(el('[data-testid="http-code-unresolved"]')).not.toBeNull());

    expect(el('[data-testid="http-code-unresolved"]').textContent).toContain('{{tenant}}');
    expect(el('[data-testid="http-code-notice"]').textContent).toContain(
      'wget n’envoie pas de formulaire multipart',
    );
  });

  it('copies the code, and saves it as a snippet in its language, tagged http', async () => {
    el('[data-testid="http-code-copy"]').click();
    await vi.waitFor(() => expect(clipboard.content).toContain('curl --url'));

    el('[data-testid="http-code-save"]').click();
    await fixture.whenStable();
    expect(el<HTMLInputElement>('[data-testid="save-as-note-tags"]').value).toBe('http');
  });
});
