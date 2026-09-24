import { TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';
import { ChangelogService, RELEASES_URL } from './changelog.service';

/**
 * Not mocked, for the reason its neighbour `app-info.service.spec.ts` already gives:
 * `vi.mock` on a Tauri package is unreliable, and jsdom gives the no-bridge case free —
 * which is the one decision this service actually makes.
 */
describe('ChangelogService', () => {
  let service: ChangelogService;

  beforeEach(() => {
    TestBed.resetTestingModule();
    TestBed.configureTestingModule({});
    service = TestBed.inject(ChangelogService);
  });

  /**
   * The command returns no `Result` — reading a string baked in with `include_str!`
   * cannot fail — so the only failure is having no bridge, and it has to reach the dialog.
   * Swallowed into an empty list it would read as "this release changed nothing".
   */
  it('rejects without a bridge rather than answering an empty changelog', async () => {
    await expect(service.load()).rejects.toThrow();
  });

  it('lets a refused open reach its caller too', async () => {
    await expect(service.openReleases()).rejects.toThrow();
  });

  /**
   * Outside the scope declared for `opener:allow-open-url` the call is refused at
   * runtime with nothing on screen to explain it. The scope itself is asserted against the
   * shipped capability file in `app_info.rs`, on the side that owns it.
   */
  it('points at the releases page of the repository the binary declares', () => {
    expect(RELEASES_URL).toBe('https://github.com/vmillet-dev/devnotes-rs/releases');
  });
});
