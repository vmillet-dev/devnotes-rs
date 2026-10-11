import { describe, expect, it } from 'vitest';
import {
  DEFAULT_TRANSPORT,
  fromSettings,
  requestBadge,
  shortBadge,
  toDocument,
  toInherited,
  toParts,
  toSettings,
  bodyOfKind,
  setBodyAside,
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

  /** « Passer à Aucun » on a `GET` would otherwise cost the text it warns about. */
  it('sets a body aside when another kind is chosen, and hands it back as it was', () => {
    const form = {
      kind: 'form',
      fields: [{ enabled: true, key: 'q', value: 'a', description: '' }],
    } as const;
    let left = setBodyAside({}, form);
    expect(bodyOfKind('none', left)).toEqual({ kind: 'none' });
    expect(bodyOfKind('form', left)).toBe(form);

    // JSON or plain, a text is one text: the last one typed, under either kind.
    left = setBodyAside(left, { kind: 'json', text: '{"a":1}' });
    expect(bodyOfKind('text', left)).toEqual({ kind: 'text', text: '{"a":1}' });
    left = setBodyAside(left, { kind: 'text', text: 'bonjour' });
    expect(bodyOfKind('json', left)).toEqual({ kind: 'json', text: 'bonjour' });
    expect(bodyOfKind('form', left)).toBe(form);
  });

  it('starts a kind never chosen empty', () => {
    expect(
      ['none', 'json', 'text', 'form', 'multipart', 'binary'].map((kind) => bodyOfKind(kind as never)),
    ).toEqual([
      { kind: 'none' },
      { kind: 'json', text: '' },
      { kind: 'text', text: '' },
      { kind: 'form', fields: [] },
      { kind: 'multipart', parts: [] },
      { kind: 'binary', path: '' },
    ]);
  });

  it('names a request by its method, or by its kind', () => {
    expect(requestBadge({ kind: 'http', method: 'PATCH' })).toBe('PATCH');
    expect(requestBadge({ kind: 'graphql', method: 'POST' })).toBe('QUERY');
    expect(requestBadge({ kind: 'websocket', method: 'GET' })).toBe('WS');
  });

  /** Where room is counted, the rail at 160px and a tab: three letters for the long methods. */
  it('cuts the long methods to three letters, and leaves the others whole', () => {
    expect(shortBadge('DELETE')).toBe('DEL');
    expect(shortBadge('OPTIONS')).toBe('OPT');
    expect(['GET', 'POST', 'PATCH', 'QUERY', 'WS'].map(shortBadge)).toEqual([
      'GET',
      'POST',
      'PATCH',
      'QUERY',
      'WS',
    ]);
  });
});
