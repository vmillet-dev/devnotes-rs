import { TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';
import { FakeAppWindow } from '@testing/fake-app-window';
import { APP_WINDOW_ADAPTER, AppWindowService } from './app-window.service';

describe('AppWindowService', () => {
  let adapter: FakeAppWindow;
  let service: AppWindowService;

  beforeEach(() => {
    TestBed.resetTestingModule();
    adapter = new FakeAppWindow();
    TestBed.configureTestingModule({
      providers: [{ provide: APP_WINDOW_ADAPTER, useValue: adapter }],
    });
    service = TestBed.inject(AppWindowService);
  });

  it('hides the window without quitting', async () => {
    await service.hide();

    expect(adapter.hidden).toBe(1);
    expect(adapter.exitedWith).toBeNull();
  });

  it('quits with a success code', async () => {
    await service.quit();

    expect(adapter.exitedWith).toBe(0);
  });

  it('rebuilds the front end without touching the process', () => {
    service.reload();

    expect(adapter.reloaded).toBe(1);
    expect(adapter.exitedWith).toBeNull();
  });

  it('swallows a failure rather than surfacing one nothing can act on', async () => {
    adapter.throwOnHide = new Error('no bridge');
    adapter.throwOnExit = new Error('no bridge');

    await expect(service.hide()).resolves.toBeUndefined();
    await expect(service.quit()).resolves.toBeUndefined();
  });
});
