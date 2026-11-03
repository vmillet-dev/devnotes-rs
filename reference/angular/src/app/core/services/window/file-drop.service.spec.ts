import { TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';
import { Unlisten } from '@core/utils/subscription.util';
import { FILE_DROP_SUBSCRIBER, FileDropService } from './file-drop.service';

/**
 * Stands in for `onDragDropEvent`, which only Rust can raise: the WebView never sees a
 * dropped file. `resolve` is held back so a spec can subscribe, cancel, and only then
 * let the subscription land — the window the service exists to close.
 */
class FakeDrops {
  handler: ((paths: readonly string[]) => void) | null = null;
  stopped = 0;

  private settle: (() => void) | null = null;

  holdBack = false;

  readonly subscribe = (handler: (paths: readonly string[]) => void): Promise<Unlisten> => {
    return new Promise<Unlisten>((resolve) => {
      const land = (): void => {
        this.handler = handler;
        resolve(() => {
          this.stopped += 1;
          this.handler = null;
        });
      };

      if (this.holdBack) {
        this.settle = land;
      } else {
        land();
      }
    });
  };

  /** Lets a held-back subscription finish arriving. */
  async land(): Promise<void> {
    this.settle?.();
    this.settle = null;
    await Promise.resolve();
  }

  drop(...paths: string[]): void {
    this.handler?.(paths);
  }
}

describe('FileDropService', () => {
  let service: FileDropService;
  let drops: FakeDrops;

  beforeEach(() => {
    TestBed.resetTestingModule();
    drops = new FakeDrops();
    TestBed.configureTestingModule({
      providers: [{ provide: FILE_DROP_SUBSCRIBER, useValue: drops.subscribe }],
    });
    service = TestBed.inject(FileDropService);
  });

  it('hands over every path of one drop, in one call', async () => {
    const dropped: (readonly string[])[] = [];
    service.on((paths) => dropped.push(paths));
    await Promise.resolve();

    drops.drop('C:/a.png', 'C:/b.pdf');

    // One call carrying two files, not two calls: a drop is one gesture.
    expect(dropped).toEqual([['C:/a.png', 'C:/b.pdf']]);
  });

  it('stops delivering once the subscription is dropped', async () => {
    const dropped: (readonly string[])[] = [];
    const stop = service.on((paths) => dropped.push(paths));
    await Promise.resolve();

    stop();
    drops.drop('C:/a.png');

    expect(drops.stopped).toBe(1);
    expect(dropped).toEqual([]);
  });

  /**
   * The window `subscribeCancellable` exists for: the subscription only lands on the
   * next turn, and a caller destroyed before then would otherwise stay subscribed for
   * the rest of the session.
   */
  it('cancels a subscription that had not landed yet', async () => {
    drops.holdBack = true;
    const dropped: (readonly string[])[] = [];

    const stop = service.on((paths) => dropped.push(paths));
    stop();
    await drops.land();

    expect(drops.stopped).toBe(1);
    drops.drop('C:/a.png');
    expect(dropped).toEqual([]);
  });
});
