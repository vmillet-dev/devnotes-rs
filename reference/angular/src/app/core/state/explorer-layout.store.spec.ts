import { TestBed } from '@angular/core/testing';
import { describe, expect, it } from 'vitest';
import { DEFAULT_CODE_SHARE, ExplorerLayoutStore } from './explorer-layout.store';

describe('ExplorerLayoutStore', () => {
  it('starts one at a time, the code shown, and keeps what it is told for the session', () => {
    const layout = TestBed.inject(ExplorerLayoutStore);
    expect([layout.sideBySide(), layout.codeHidden(), layout.codeShare()]).toEqual([
      false,
      false,
      DEFAULT_CODE_SHARE,
    ]);

    layout.toggleSideBySide();
    layout.toggleCode();
    layout.setCodeShare(62.5);
    expect([layout.sideBySide(), layout.codeHidden(), layout.codeShare()]).toEqual([true, true, 62.5]);

    // The line asks for its place back with no share at all.
    layout.setCodeShare(0);
    expect(layout.codeShare()).toBe(DEFAULT_CODE_SHARE);
  });
});
