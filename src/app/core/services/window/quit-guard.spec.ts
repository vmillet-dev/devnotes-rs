import { TestBed } from '@angular/core/testing';
import { describe, expect, it } from 'vitest';
import { QuitGuard } from './quit-guard';

describe('QuitGuard', () => {
  it('waits for every task, and a failing one does not stop the rest', async () => {
    const guard = TestBed.inject(QuitGuard);
    const done: string[] = [];
    guard.register(async () => {
      throw new Error('the save failed');
    });
    guard.register(async () => {
      done.push('layout');
    });

    await guard.settle();

    expect(done).toEqual(['layout']);
  });

  it('forgets a task that was taken back', async () => {
    const guard = TestBed.inject(QuitGuard);
    let ran = 0;
    const release = guard.register(async () => {
      ran += 1;
    });

    release();
    await guard.settle();

    expect(ran).toBe(0);
  });
});
