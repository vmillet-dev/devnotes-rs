import { signal } from '@angular/core';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { IpcError } from '@core/ipc/ipc.error';
import { ErrorNotifier, ipcNotice } from './error-notifier.service';

const FALLBACK = { key: 'errors.noteSaveFailed' };

function failure(code: string, params: Record<string, string> = {}): IpcError {
  return new IpcError('update_note', { code, params, detail: 'détail technique' });
}

describe('ipcNotice', () => {
  it('prefers the message naming the cause over the one naming the attempted action', () => {
    const notice = ipcNotice(failure('noteNotFound', { id: 'n-1' }), FALLBACK);

    expect(notice.ref.key).toBe('errors.noteGone');
  });

  it('falls back to the attempted action when the cause adds nothing useful', () => {
    const notice = ipcNotice(failure('storage'), FALLBACK);

    expect(notice.ref.key).toBe('errors.noteSaveFailed');
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

  it('keeps the technical detail as secondary text in every case', () => {
    expect(ipcNotice(failure('noteNotFound'), FALLBACK).detail).toBe(
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
