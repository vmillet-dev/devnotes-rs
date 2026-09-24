import { TestBed } from '@angular/core/testing';
import { check } from '@tauri-apps/plugin-updater';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { DownloadProgress, UPDATE_CHECK, UpdaterService } from './updater.service';

/** What is pinned here is the download arithmetic and the lifetime of the native handle. */

/** Download event as the plugin emits it, reduced to the fields that are read. */
type DownloadEvent =
  | { event: 'Started'; data: { contentLength?: number } }
  | { event: 'Progress'; data: { chunkLength: number } }
  | { event: 'Finished' };

/** Stand-in for the plugin's `Update`, a native resource that has to be closed. */
function fakeUpdate(events: DownloadEvent[] = []) {
  return {
    version: '0.5.0',
    currentVersion: '0.4.1',
    body: 'Notes',
    closed: 0,
    downloadAndInstall: vi.fn(async (onEvent: (event: DownloadEvent) => void) => {
      for (const event of events) onEvent(event);
    }),
    close: vi.fn(async function (this: { closed: number }) {
      this.closed += 1;
    }),
  };
}

describe('UpdaterService', () => {
  let service: UpdaterService;
  let offered: ReturnType<typeof fakeUpdate> | null;

  /** What the next `check()` finds, retained the way the real one retains it. */
  async function retain(update: ReturnType<typeof fakeUpdate>): Promise<void> {
    offered = update;
    await service.check();
  }

  beforeEach(() => {
    offered = null;
    TestBed.configureTestingModule({
      providers: [{ provide: UPDATE_CHECK, useValue: async () => offered }],
    });
    service = TestBed.inject(UpdaterService);
  });

  it('asks the plugin when nothing stands in for it', () => {
    TestBed.resetTestingModule();

    expect(TestBed.inject(UPDATE_CHECK)).toBe(check);
  });

  it('refuses to install what no check has retained', async () => {
    await expect(service.install(() => undefined)).rejects.toThrow();
  });

  it('turns the download events into a fraction of the total', async () => {
    await retain(
      fakeUpdate([
        { event: 'Started', data: { contentLength: 400 } },
        { event: 'Progress', data: { chunkLength: 100 } },
        { event: 'Progress', data: { chunkLength: 100 } },
        { event: 'Finished' },
      ]),
    );
    const progress: DownloadProgress[] = [];

    await service.install((value) => progress.push(value));

    expect(progress).toEqual([null, 0.25, 0.5, 1]);
  });

  it('leaves the progress undetermined when the server announces no size', async () => {
    await retain(
      fakeUpdate([
        { event: 'Started', data: {} },
        { event: 'Progress', data: { chunkLength: 100 } },
      ]),
    );
    const progress: DownloadProgress[] = [];

    await service.install((value) => progress.push(value));

    expect(progress).toEqual([null, null]);
  });

  it('reports no progress at all for a download that never starts', async () => {
    await retain(fakeUpdate());
    const progress: DownloadProgress[] = [];

    await service.install((value) => progress.push(value));

    expect(progress).toEqual([]);
  });

  it('lets go of the update once it is installed', async () => {
    await retain(fakeUpdate([{ event: 'Finished' }]));
    await service.install(() => undefined);

    await expect(service.install(() => undefined)).rejects.toThrow();
  });

  it('closes the update without installing it when it is discarded', async () => {
    const update = fakeUpdate();
    await retain(update);

    await service.discard();

    expect(update.closed).toBe(1);
    expect(update.downloadAndInstall).not.toHaveBeenCalled();
  });

  it('has nothing to close when no update is pending', async () => {
    const update = fakeUpdate();
    await retain(update);
    await service.discard();

    await expect(service.discard()).resolves.toBeUndefined();
    expect(update.closed).toBe(1);
  });
});
