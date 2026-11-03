import { signal } from '@angular/core';
import { TestBed } from '@angular/core/testing';
import { TranslocoService } from '@jsverse/transloco';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { IpcError } from '@core/ipc/ipc.error';
import { provideTranslocoTesting } from '@testing/provide-transloco-testing';
import { ErrorNotifier, errorKeyOf, ipcNotice } from './error-notifier.service';

const FALLBACK = { key: 'errors.noteSaveFailed' };

function failure(code: string, params: Record<string, string> = {}): IpcError {
  return new IpcError('update_note', { code, params, detail: 'détail technique' });
}

/**
 * Both sentences used to state the defaults — « 30 secondes », « plus de 10 » — whatever the
 * request was sent with. No number is written in a translation any more.
 */
describe('a limit a send ran into', () => {
  let transloco: TranslocoService;

  beforeEach(() => {
    TestBed.configureTestingModule({ providers: [provideTranslocoTesting()] });
    transloco = TestBed.inject(TranslocoService);
    // The testing loader answers at once: both languages are there before the first sentence.
    for (const lang of ['fr', 'en']) transloco.load(lang).subscribe();
  });

  function said(code: string, params: Record<string, string>, lang = 'fr'): string {
    const { ref } = ipcNotice(failure(code, params), FALLBACK);
    return transloco.translate(ref.key, ref.params, lang);
  }

  it.each([
    [{ amount: '1', unit: 's', source: 'request' }, 'Pas de réponse au bout de 1 s.'],
    [
      { amount: '1500', unit: 'ms', source: 'default' },
      'Pas de réponse au bout de 1500 ms (réglage par défaut).',
    ],
    [
      { amount: '2', unit: 'min', source: 'folder', name: 'un ss-dossier' },
      'Pas de réponse au bout de 2 min (réglage hérité du dossier « un ss-dossier »).',
    ],
    [
      { amount: '30', unit: 's', source: 'collection', name: 'API' },
      'Pas de réponse au bout de 30 s (réglage hérité de la collection « API »).',
    ],
  ])('says the delay that ran out, and who set it: %o', (params, sentence) => {
    expect(said('httpTimeout', params)).toBe(sentence);
  });

  it.each([
    [{ count: '1', source: 'request' }, 'Trop de redirections : plus d’une.'],
    [{ count: '4', source: 'request' }, 'Trop de redirections : plus de 4.'],
    [{ count: '0', source: 'request' }, 'Redirection refusée : le réglage n’en permet aucune.'],
    [{ count: '10', source: 'default' }, 'Trop de redirections : plus de 10 (réglage par défaut).'],
    [
      { count: '1', source: 'collection', name: 'API' },
      'Trop de redirections : plus d’une (réglage hérité de la collection « API »).',
    ],
    [
      { count: '3', source: 'folder', name: 'Auth' },
      'Trop de redirections : plus de 3 (réglage hérité du dossier « Auth »).',
    ],
  ])('says how many redirections were allowed: %o', (params, sentence) => {
    expect(said('httpTooManyRedirects', params)).toBe(sentence);
  });

  it('says both in English too', () => {
    expect(said('httpTimeout', { amount: '1', unit: 's', source: 'request' }, 'en')).toBe(
      'No answer after 1 s.',
    );
    expect(said('httpTimeout', { amount: '5', unit: 's', source: 'folder', name: 'Auth' }, 'en')).toBe(
      'No answer after 5 s (setting inherited from the folder “Auth”).',
    );
    expect(said('httpTooManyRedirects', { count: '1', source: 'request' }, 'en')).toBe(
      'Too many redirects: more than one.',
    );
    expect(said('httpTooManyRedirects', { count: '10', source: 'default' }, 'en')).toBe(
      'Too many redirects: more than 10 (default setting).',
    );
    expect(said('httpTooManyRedirects', { count: '0', source: 'collection', name: 'API' }, 'en')).toBe(
      'Redirect refused: the setting allows none (setting inherited from the collection “API”).',
    );
  });

  /** It used to be blamed on the URL, whoever had written the header. */
  it('names a header that cannot be sent, and the folder or the collection it comes from', () => {
    expect(said('httpHeaderName', { header: 'X-Recette: 1', source: 'folder', name: 'Recette' })).toBe(
      'L’en-tête « X-Recette: 1 », hérité du dossier « Recette », ne peut pas être envoyé : son nom n’est pas valide.',
    );
    expect(said('httpHeaderName', { header: 'X Recette', source: 'request' })).toBe(
      'L’en-tête « X Recette » ne peut pas être envoyé : son nom n’est pas valide.',
    );
    expect(said('httpHeaderValue', { header: 'X-Trace', source: 'collection', name: 'API' }, 'en')).toBe(
      'The header “X-Trace”, inherited from the collection “API”, cannot be sent: its value is not valid.',
    );
  });

  it("reads an error kept without its values as the request's own", () => {
    expect(errorKeyOf('httpTimeout')).toBe('errors.httpTimeout.request');
    expect(errorKeyOf('httpRefused', { source: 'folder' })).toBe('errors.httpRefused');
    expect(errorKeyOf('storage')).toBeNull();
  });
});

describe('ipcNotice', () => {
  it('prefers the message naming the cause over the one naming the attempted action', () => {
    const notice = ipcNotice(failure('noteNotFound', { id: 'n-1' }), FALLBACK);

    expect(notice.ref.key).toBe('errors.noteGone');
  });

  it('falls back to the attempted action when the cause adds nothing useful', () => {
    const notice = ipcNotice(failure('storage'), FALLBACK);

    expect(notice.ref.key).toBe('errors.noteSaveFailed');
  });

  it('never shows a field of the code: a refused value says what the action was', () => {
    const notice = ipcNotice(failure('invalidInput', { field: 'byteSize' }), FALLBACK);

    expect(notice.ref.key).toBe('errors.noteSaveFailed');
  });

  it('names a file too large to attach, and the limit', () => {
    const notice = ipcNotice(failure('fileTooLarge', { name: 'capture.mov', limit: '10' }), FALLBACK);

    expect(notice.ref).toEqual({ key: 'errors.fileTooLarge', params: { name: 'capture.mov', limit: '10' } });
  });

  it('falls back when Tauri itself rejected, which carries no code', () => {
    const notice = ipcNotice(new IpcError('update_note', 'command not found'), FALLBACK);

    expect(notice.ref.key).toBe('errors.noteSaveFailed');
  });

  it('falls back for a plain error that never crossed the bridge', () => {
    const notice = ipcNotice(new Error('boom'), FALLBACK);

    expect(notice.ref.key).toBe('errors.noteSaveFailed');
    expect(notice.detail).toBe('boom');
  });

  it('carries the backend parameters through for interpolation', () => {
    const notice = ipcNotice(failure('duplicateSpaceName', { name: 'Perso' }), FALLBACK);

    expect(notice.ref).toEqual({ key: 'errors.spaceNameTaken', params: { name: 'Perso' } });
  });

  it('lets the backend parameters win over the caller defaults', () => {
    const notice = ipcNotice(failure('duplicateSpaceName', { name: 'Perso' }), FALLBACK, {
      name: 'saisi',
    });

    expect(notice.ref.params).toEqual({ name: 'Perso' });
  });

  it('keeps the caller default when the backend sent no parameter to interpolate', () => {
    const notice = ipcNotice(failure('duplicateSpaceName'), FALLBACK, { name: 'saisi' });

    expect(notice.ref.params).toEqual({ name: 'saisi' });
  });

  /** « Ce fichier ne contient aucune ligne… » came with « Tauri command … failed: The file holds no row » (HSC-17). */
  it('says a named cause alone, and keeps the detail for a fallback and a file not reached', () => {
    expect(ipcNotice(failure('noteNotFound'), FALLBACK).detail).toBeUndefined();
    expect(ipcNotice(failure('httpRunData', { fault: 'unreadable' }), FALLBACK)).toEqual({
      ref: { key: 'errors.httpRunData.unreadable', params: { fault: 'unreadable' } },
    });
    expect(ipcNotice(failure('fileAccess'), FALLBACK).detail).toBe(
      'Tauri command "update_note" failed: détail technique',
    );
    expect(ipcNotice(failure('storage'), FALLBACK).detail).toBe(
      'Tauri command "update_note" failed: détail technique',
    );
  });

  it('ignores a code this build does not know rather than trusting it blindly', () => {
    const notice = ipcNotice(failure('quantumFluctuation'), FALLBACK);

    expect(notice.ref.key).toBe('errors.noteSaveFailed');
  });
});

describe('ErrorNotifier', () => {
  let notifier: ErrorNotifier;

  beforeEach(() => {
    notifier = new ErrorNotifier();
    vi.spyOn(console, 'error').mockImplementation(() => undefined);
  });

  it('answers what the action answered', async () => {
    await expect(notifier.attempt('errors.noteSaveFailed', async () => 42)).resolves.toBe(42);
    expect(notifier.notice()).toBeNull();
  });

  it('reports a rejection and answers null', async () => {
    const answer = await notifier.attempt('errors.noteSaveFailed', () => Promise.reject(new Error('boom')));

    expect(answer).toBeNull();
    expect(notifier.notice()?.ref.key).toBe('errors.noteSaveFailed');
  });

  /** A thunk that throws before it has a promise to return, like a mapper refusing a date. */
  it('reports a synchronous throw as a failure rather than letting it escape', async () => {
    const answer = await notifier.attempt('errors.noteSaveFailed', () => {
      throw new Error('before any promise');
    });

    expect(answer).toBeNull();
    expect(notifier.notice()?.detail).toBe('before any promise');
  });

  it('raises the flag for the length of the call, and lowers it on failure too', async () => {
    const busy = signal(false);
    const seen: boolean[] = [];

    await notifier.attemptWhile(busy, 'errors.noteSaveFailed', async () => seen.push(busy()));
    await notifier.attemptWhile(busy, 'errors.noteSaveFailed', () => {
      seen.push(busy());
      return Promise.reject(new Error('boom'));
    });

    expect(seen).toEqual([true, true]);
    expect(busy()).toBe(false);
  });
});
