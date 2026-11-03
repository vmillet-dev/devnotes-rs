import { ApplicationRef, resource, signal } from '@angular/core';
import { TestBed } from '@angular/core/testing';
import { describe, expect, it } from 'vitest';
import { retained } from './retained.util';

describe('retained', () => {
  it('keeps the last value while a reload is in flight, and takes the next one when it lands', async () => {
    TestBed.configureTestingModule({});
    const key = signal(1);
    let land: (value: string) => void = () => undefined;
    const loaded = TestBed.runInInjectionContext(() =>
      resource({
        params: () => key(),
        loader: ({ params }) =>
          params === 1 ? Promise.resolve('first') : new Promise<string>((resolve) => (land = resolve)),
      }),
    );
    const kept = retained(loaded);

    await TestBed.inject(ApplicationRef).whenStable();
    expect(kept()).toBe('first');

    key.set(2);
    TestBed.tick();
    expect(loaded.isLoading()).toBe(true);
    expect(kept()).toBe('first');

    land('second');
    await TestBed.inject(ApplicationRef).whenStable();
    expect(kept()).toBe('second');
  });

  it('answers null before anything has loaded', () => {
    TestBed.configureTestingModule({});
    const loaded = TestBed.runInInjectionContext(() =>
      resource({ loader: () => new Promise<string>(() => undefined) }),
    );

    expect(retained(loaded)()).toBeNull();
  });
});
