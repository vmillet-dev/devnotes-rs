import { TestBed } from '@angular/core/testing';
import { TranslocoService } from '@jsverse/transloco';
import { firstValueFrom } from 'rxjs';
import { beforeEach, describe, expect, it } from 'vitest';
import { FakeDesktopNotifications } from '@testing/fake-desktop-notifications';
import { provideTranslocoTesting } from '@testing/provide-transloco-testing';
import { DESKTOP_NOTIFICATION_ADAPTER, DesktopNotifier } from './desktop-notifier.service';

describe('DesktopNotifier', () => {
  let notifier: DesktopNotifier;
  let desktop: FakeDesktopNotifications;

  async function createNotifier(permission: NotificationPermission = 'granted'): Promise<void> {
    desktop = new FakeDesktopNotifications(permission);
    TestBed.resetTestingModule();
    TestBed.configureTestingModule({
      providers: [provideTranslocoTesting(), { provide: DESKTOP_NOTIFICATION_ADAPTER, useValue: desktop }],
    });
    // The text leaves for the operating system already formed, so the language has to be in.
    await firstValueFrom(TestBed.inject(TranslocoService).load('fr'));
    notifier = TestBed.inject(DesktopNotifier);
  }

  beforeEach(async () => {
    await createNotifier();
  });

  it('sends what the keys say, translated and interpolated', async () => {
    expect(
      await notifier.notify(
        { key: 'palette.copiedTitle' },
        { key: 'palette.copiedNote', params: { title: 'Reset the dev database' } },
      ),
    ).toBe(true);

    expect(desktop.sent[0]?.title).toBe('Copié');
    expect(desktop.sent[0]?.body).toContain('Reset the dev database');
  });

  /** Whatever it was asked to say has already happened; a refusal is not a failure. */
  it('answers no rather than throwing when the desktop refuses', async () => {
    await createNotifier('denied');

    expect(await notifier.notify({ key: 'palette.copiedTitle' }, { key: 'palette.copiedUntitled' })).toBe(
      false,
    );
    expect(desktop.sent).toEqual([]);
  });

  /** Outside Tauri the plugin throws rather than answering no, and that is the same thing. */
  it('answers no rather than throwing when there is no plugin at all', async () => {
    desktop.failNext = new Error('no plugin');

    expect(await notifier.notify({ key: 'palette.copiedTitle' }, { key: 'palette.copiedUntitled' })).toBe(
      false,
    );
  });

  /** Once. A prompt on every copy is worse than no acknowledgement at all. */
  it('asks the desktop for permission once, not once per toast', async () => {
    await createNotifier('default');
    await notifier.notify({ key: 'palette.copiedTitle' }, { key: 'palette.copiedUntitled' });
    await notifier.notify({ key: 'palette.copiedTitle' }, { key: 'palette.copiedUntitled' });

    expect(desktop.asked).toBe(1);
  });
});
