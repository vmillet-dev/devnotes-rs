import { TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { APP_INFO, AppInfoService } from './app-info.service';

/** Not mocked: `vi.mock` on a Tauri package is unreliable, and jsdom gives the no-bridge case free. */
describe('AppInfoService', () => {
  beforeEach(() => {
    TestBed.resetTestingModule();
    TestBed.configureTestingModule({});
  });

  it('reads null outside a Tauri runtime rather than making a version up', async () => {
    const info = TestBed.inject(AppInfoService);

    expect(info.version()).toBeNull();
    await vi.waitFor(() => expect(info.version()).toBeNull());
  });

  it('carries what Cargo.toml declares, down to the capability scope', () => {
    // Outside the scope declared for `opener:allow-open-url` the call is refused at
    // runtime, and nothing says so before then.
    expect(APP_INFO.repository).toBe('https://github.com/vmillet-dev/devnotes-rs');
    expect(APP_INFO.name).toBe('DevNotes');
    expect(APP_INFO.author).toBe('Valentin MILLET');
    expect(APP_INFO.authorHandle).toBe('@vmillet-dev');
  });
});
