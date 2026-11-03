import { TestBed } from '@angular/core/testing';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { IpcError } from '@core/ipc/ipc.error';
import { AppErrorHandler } from './app-error-handler';
import { ErrorNotifier } from './error-notifier.service';

describe('AppErrorHandler', () => {
  let handler: AppErrorHandler;
  let notifier: ErrorNotifier;

  beforeEach(() => {
    TestBed.resetTestingModule();
    TestBed.configureTestingModule({ providers: [AppErrorHandler] });
    vi.spyOn(console, 'error').mockImplementation(() => undefined);
    handler = TestBed.inject(AppErrorHandler);
    notifier = TestBed.inject(ErrorNotifier);
  });

  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('reports an IPC failure with the failing command name', () => {
    handler.handleError(new IpcError('query_notes', 'no such command'));

    expect(notifier.notice()?.ref).toEqual({ key: 'errors.ipcFailed', params: { command: 'query_notes' } });
  });

  it('reports any other error as unexpected, keeping the message as detail', () => {
    handler.handleError(new Error('boom'));

    expect(notifier.notice()?.ref.key).toBe('errors.unexpected');
    expect(notifier.notice()?.detail).toBe('boom');
  });

  it('handles a thrown non-Error value', () => {
    handler.handleError('plain string');

    expect(notifier.notice()?.detail).toBe('plain string');
  });

  it('still logs to the console for debugging', () => {
    const error = new Error('boom');

    handler.handleError(error);

    expect(console.error).toHaveBeenCalledWith(error);
  });
});

describe('ErrorNotifier', () => {
  beforeEach(() => {
    TestBed.resetTestingModule();
  });

  it('holds only the most recent notice', () => {
    const notifier = TestBed.inject(ErrorNotifier);

    notifier.notify({ ref: { key: 'errors.unexpected' } });
    notifier.notify({ ref: { key: 'errors.noteSaveFailed' } });

    expect(notifier.notice()?.ref.key).toBe('errors.noteSaveFailed');
  });

  it('clears the notice on dismiss', () => {
    const notifier = TestBed.inject(ErrorNotifier);
    notifier.notify({ ref: { key: 'errors.unexpected' } });

    notifier.dismiss();

    expect(notifier.notice()).toBeNull();
  });
});
