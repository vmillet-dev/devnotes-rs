import { TestBed } from '@angular/core/testing';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { CLOCK_TICK_MS, ClockService } from './clock.service';

describe('ClockService', () => {
  beforeEach(() => {
    // This spec is the one place that needs timer control as well as Date:
    // it asserts on the interval itself, not on rendering.
    vi.useFakeTimers();
    vi.setSystemTime(new Date('2026-01-01T12:00:00Z'));
    TestBed.resetTestingModule();
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('starts at the current time', () => {
    const clock = TestBed.inject(ClockService);

    expect(clock.now()).toEqual(new Date('2026-01-01T12:00:00Z'));
  });

  it('advances on each tick, so relative-time labels age on their own', () => {
    const clock = TestBed.inject(ClockService);

    vi.advanceTimersByTime(CLOCK_TICK_MS);

    expect(clock.now()).toEqual(new Date('2026-01-01T12:00:30Z'));
  });

  it('keeps advancing over several ticks', () => {
    const clock = TestBed.inject(ClockService);

    vi.advanceTimersByTime(CLOCK_TICK_MS * 4);

    expect(clock.now()).toEqual(new Date('2026-01-01T12:02:00Z'));
  });

  it('stops ticking once the injector is destroyed', () => {
    const clock = TestBed.inject(ClockService);
    const initial = clock.now();

    TestBed.resetTestingModule();
    vi.advanceTimersByTime(CLOCK_TICK_MS * 3);

    expect(clock.now()).toEqual(initial);
  });
});
