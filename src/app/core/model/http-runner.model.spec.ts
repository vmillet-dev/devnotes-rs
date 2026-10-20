import { describe, expect, it } from 'vitest';
import { sampleTree } from '@testing/http-tree.fixture';
import {
  RunResult,
  byIteration,
  changes,
  failed,
  moved,
  rowsUnder,
  stepped,
  summaryOf,
} from './http-runner.model';

function result(name: string, overrides: Partial<RunResult> = {}): RunResult {
  return {
    iteration: 0,
    requestId: name,
    name,
    path: [],
    kind: 'http',
    method: 'GET',
    status: 200,
    reason: 'OK',
    millis: 10,
    tests: [],
    error: null,
    stop: null,
    skipped: false,
    response: null,
    ...overrides,
  };
}

describe('the runner’s model', () => {
  it('lists the requests under a collection or a folder in the tree’s order, with their folders', () => {
    expect(
      rowsUnder(sampleTree(), { kind: 'collection', id: 'API' }).map((row) => [row.path.join('/'), row.name]),
    ).toEqual([
      ['Auth', 'Login'],
      ['Auth', 'Logout'],
      ['Factures/Archives', 'Old'],
      ['', 'Health'],
    ]);
    expect(rowsUnder(sampleTree(), { kind: 'folder', id: 'Factures' }).map((row) => row.name)).toEqual([
      'Old',
    ]);
    expect(rowsUnder(sampleTree(), { kind: 'folder', id: 'nowhere' })).toEqual([]);
  });

  it('says which requests a protected environment asks about', () => {
    expect(changes({ kind: 'http', method: 'GET' })).toBe(false);
    expect(changes({ kind: 'http', method: 'POST' })).toBe(true);
    expect(changes({ kind: 'websocket', method: 'GET' })).toBe(true);
  });

  it('fails a result on an error, a stop or a failed test, never when skipped, and sums them up', () => {
    const passing = result('A', { tests: [{ name: 'ok', passed: true, skipped: false }] });
    const failing = result('B', { iteration: 1, tests: [{ name: 'ko', passed: false, skipped: false }] });
    const refused = result('C', { iteration: 1, status: null, error: { code: 'httpRefused', params: {} } });
    const skipped = result('D', { skipped: true, tests: [{ name: 'ko', passed: false, skipped: false }] });
    expect([passing, failing, refused, skipped].map(failed)).toEqual([false, true, true, false]);
    expect(summaryOf([passing, failing, refused, skipped])).toEqual({
      failures: 2,
      requests: 4,
      passed: 1,
      tests: 3,
    });
    expect(
      byIteration([passing, failing, refused]).map((group) => [group.iteration, group.results.length]),
    ).toEqual([
      [0, 1],
      [1, 2],
    ]);
  });

  it('folds the steps of a run into its results, and moves a row of the order', () => {
    const one = result('A');
    expect(stepped([], { kind: 'ended', result: one })).toEqual([one]);
    expect(stepped([one], { kind: 'started', iteration: 0, index: 1, name: 'B' })).toEqual([one]);
    expect(moved(['a', 'b', 'c'], 2, 0)).toEqual(['c', 'a', 'b']);
    expect(moved(['a', 'b', 'c'], 0, 9)).toEqual(['b', 'c', 'a']);
    const same = ['a'];
    expect(moved(same, 0, 0)).toBe(same);
    expect(moved(same, 4, 0)).toBe(same);
  });
});
