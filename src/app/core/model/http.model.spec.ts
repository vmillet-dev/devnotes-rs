import { describe, expect, it } from 'vitest';
import {
  DEFAULT_TRANSPORT,
  fromSettings,
  requestBadge,
  toDocument,
  toInherited,
  toParts,
  toSettings,
} from './http.model';

describe('the HTTP wire shapes', () => {
  it('fills what a document left out, and gives every row and part its defaults', () => {
    const parts = toParts({
      url: '/pay',
      headers: [{ key: 'Accept' }],
      body: { kind: 'multipart', parts: [{ key: 'scan', value: 'C:/a.pdf', file: true }, {}] },
    });

    expect(parts).toEqual({
      url: '/pay',
      params: [],
      headers: [{ enabled: true, key: 'Accept', value: '', description: '' }],
      body: {
        kind: 'multipart',
        parts: [
          { enabled: true, key: 'scan', value: 'C:/a.pdf', description: '', file: true },
          { enabled: true, key: '', value: '', description: '', file: false },
        ],
      },
      auth: { kind: 'inherit' },
      description: '',
      graphql: { query: '', variables: '', operationName: null, asGet: false },
      websocket: { protocols: [], messages: [] },
      transport: {},
    });
    expect(toParts({ websocket: { messages: [{ name: 'Ping' }] } }).websocket).toEqual({
      protocols: [],
      messages: [{ name: 'Ping', text: '' }],
    });
    expect(toParts({}).body).toEqual({ kind: 'none' });
    expect(toParts({ graphql: { query: 'query A { a }', asGet: true } }).graphql).toEqual({
      query: 'query A { a }',
      variables: '',
      operationName: null,
      asGet: true,
    });
  });

  it('writes each kind of body back as it reads it', () => {
    for (const body of [
      { kind: 'none' as const },
      { kind: 'json' as const, text: '{}' },
      { kind: 'text' as const, text: 'x' },
      { kind: 'binary' as const, path: 'C:/a.bin' },
      { kind: 'form' as const, fields: [{ enabled: false, key: 'q', value: '1', description: '' }] },
    ]) {
      const parts = toParts({ body });
      expect(toParts(toDocument(parts)).body).toEqual(body);
    }
  });

  it('reads settings and what is inherited, their rows filled', () => {
    expect(toSettings({})).toEqual({ auth: { kind: 'inherit' }, headers: [], transport: {} });
    expect(fromSettings({ auth: { kind: 'none' }, headers: [], transport: { timeoutMs: 500 } })).toEqual({
      auth: { kind: 'none' },
      headers: [],
      transport: { timeoutMs: 500 },
    });

    const from = { kind: 'collection' as const, id: 'api', name: 'API' };
    expect(
      toInherited({
        auth: { kind: 'none' },
        authFrom: null,
        headers: [{ header: { key: 'Accept' }, from }],
        transport: DEFAULT_TRANSPORT,
      }).headers,
    ).toEqual([{ header: { enabled: true, key: 'Accept', value: '', description: '' }, from }]);
  });

  it('names a request by its method, or by its kind', () => {
    expect(requestBadge({ kind: 'http', method: 'PATCH' })).toBe('PATCH');
    expect(requestBadge({ kind: 'graphql', method: 'POST' })).toBe('QUERY');
    expect(requestBadge({ kind: 'websocket', method: 'GET' })).toBe('WS');
  });
});
