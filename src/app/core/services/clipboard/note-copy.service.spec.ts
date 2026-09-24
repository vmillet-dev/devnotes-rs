import { TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';
import { ErrorNotifier } from '@core/services/errors/error-notifier.service';
import { FakeClipboard } from '@testing/fake-clipboard';
import { provideAppTesting } from '@testing/testing.providers';
import { NoteCopyService } from './note-copy.service';

describe('NoteCopyService', () => {
  let service: NoteCopyService;
  let clipboard: FakeClipboard;
  let notifier: ErrorNotifier;

  beforeEach(() => {
    TestBed.resetTestingModule();
    clipboard = new FakeClipboard();
    TestBed.configureTestingModule({ providers: [provideAppTesting({ clipboard })] });
    service = TestBed.inject(NoteCopyService);
    notifier = TestBed.inject(ErrorNotifier);
  });

  it('puts the text on the clipboard and says it landed', async () => {
    expect(await service.copy('psql -h prod')).toBe(true);

    expect(clipboard.content).toBe('psql -h prod');
    expect(notifier.notice()).toBeNull();
  });

  /**
   * ⚠️ The answer is what the clipboard accepted, not what was attempted: outside Tauri
   * the plugin throws, and a caller that took a copy on trust would tick a card that
   * copied nothing.
   */
  it('reports a refusal rather than answering true', async () => {
    clipboard.failNext = new Error('no clipboard');

    expect(await service.copy('psql -h prod')).toBe(false);
    expect(notifier.notice()?.ref.key).toBe('errors.copyFailed');
  });

  it('copies an empty string rather than treating it as nothing to do', async () => {
    clipboard.content = 'ce qui était là avant';

    expect(await service.copy('')).toBe(true);
    expect(clipboard.content).toBe('');
  });
});
