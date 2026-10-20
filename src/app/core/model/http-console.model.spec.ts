import { describe, expect, it } from 'vitest';
import {
  ConsoleEntry,
  consoleCounts,
  consoleKindOf,
  consoleTime,
  originKey,
  secretSegments,
} from './http-console.model';

const at = (line: ConsoleEntry['line']): ConsoleEntry => ({
  at: 0,
  sendId: 'Login#1',
  request: 'Login',
  iteration: null,
  line,
});

describe('the console’s lines', () => {
  it('files each line under traffic, logs or errors, and counts them', () => {
    const entries = [
      at({ kind: 'out', method: 'GET', url: '/', headers: [], by: null }),
      at({ kind: 'in', status: 200, reason: 'OK', millis: 3, size: 2, by: null }),
      at({ kind: 'log', level: 'log', text: 'hi', by: null }),
      at({ kind: 'failed', code: 'httpRefused', params: {}, by: null }),
      at({ kind: 'test', name: 't', expected: null, actual: null, message: 'x', by: null }),
      at({ kind: 'stopped', stop: { kind: 'error', message: 'x' } }),
      at({ kind: 'missed', path: 'body.x', variable: 'x' }),
    ];
    expect(entries.map((entry) => consoleKindOf(entry.line))).toEqual([
      'traffic',
      'traffic',
      'logs',
      'errors',
      'errors',
      'errors',
      'errors',
    ]);
    expect(consoleCounts(entries)).toEqual({ all: 7, traffic: 2, logs: 1, errors: 4 });
  });

  it('sets a secret written as its name apart from the words around it', () => {
    expect(secretSegments('Bearer {{accessToken}} for {{user}}')).toEqual([
      { text: 'Bearer ', secret: false },
      { text: '{{accessToken}}', secret: true },
      { text: ' for ', secret: false },
      { text: '{{user}}', secret: true },
    ]);
    expect(secretSegments('')).toEqual([]);
  });

  it('writes the local time to the millisecond, and says who sent a line', () => {
    expect(consoleTime(new Date(2026, 9, 20, 14, 2, 11, 98).getTime())).toBe('14:02:11.098');
    const by = { kind: 'before' as const, level: 'collection' as const, name: 'API' };
    expect(originKey({ kind: 'out', method: 'GET', url: '/', headers: [], by }, by)).toBe(
      'http.console.origins.sent',
    );
    expect(originKey({ kind: 'log', level: 'log', text: '', by }, by)).toBe('http.console.origins.before');
  });
});
