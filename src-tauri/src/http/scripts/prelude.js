// What a request script sees: `req`, `res`, `env`, `test`, `expect`, `console`, `require`, the
// timers and Postman's `pm`. Evaluated once per engine, before any script; it reaches Rust only
// through `__native`, which it takes off the global object.
(function (native) {
  'use strict';

  const global = globalThis;
  delete global.__native;

  const define = (target, name, value) =>
    Object.defineProperty(target, name, { value, writable: true, configurable: true });
  const lazy = (target, name, load) =>
    Object.defineProperty(target, name, {
      configurable: true,
      get() {
        const value = load();
        define(target, name, value);
        return value;
      },
      set(value) {
        define(target, name, value);
      },
    });

  // ── Showing a value ──────────────────────────────────────────────────────────────────────

  const SHOWN = 2000;

  function show(value) {
    if (typeof value === 'string') return JSON.stringify(value);
    if (value === undefined) return 'undefined';
    if (typeof value === 'function') return `[Function${value.name ? ` ${value.name}` : ''}]`;
    if (typeof value === 'bigint') return `${value}n`;
    if (typeof value === 'symbol') return value.toString();
    if (value instanceof Error) return `${value.name}: ${value.message}`;
    if (Number.isNaN(value)) return 'NaN';
    try {
      const text = String(JSON.stringify(value));
      return text.length > SHOWN ? `${text.slice(0, SHOWN)}…` : text;
    } catch {
      return Object.prototype.toString.call(value);
    }
  }

  const written = (value) => (typeof value === 'string' ? value : show(value));
  const line = (args) => args.map(written).join(' ');

  // What a variable holds: a text, as an environment stores it.
  function stored(value) {
    if (value === undefined || value === null) return '';
    if (typeof value === 'string') return value;
    if (typeof value === 'object') {
      try {
        return JSON.stringify(value);
      } catch {
        return String(value);
      }
    }
    return String(value);
  }

  // ── Where a script is, and where an error comes from ─────────────────────────────────────

  const state = {
    job: null,
    script: null,
    request: null,
    response: null,
    writes: [],
    tests: [],
    pending: [],
    skip: false,
    next: undefined,
    layers: [],
    local: new Map(),
  };

  // `script-3:12:5`: the third script of the job, line 12. The first frame of a script wins.
  function placeOf(error) {
    const stack = error && typeof error.stack === 'string' ? error.stack : '';
    const found = /script-(\d+):(\d+)(?::(\d+))?/.exec(stack);
    if (found === null) return { script: null, line: null };
    const script = state.job && state.job.scripts[Number(found[1])];
    return { script: script ? script.id : null, line: Number(found[2]) };
  }

  // ── console ──────────────────────────────────────────────────────────────────────────────

  const log =
    (level) =>
    (...args) =>
      native.log(JSON.stringify({ level, text: line(args), script: state.script && state.script.id }));
  define(global, 'console', {
    log: log('log'),
    info: log('info'),
    debug: log('debug'),
    warn: log('warn'),
    error: log('error'),
    trace: log('debug'),
    dir: (value) => log('log')(value),
    table: (value) => log('log')(value),
    assert: (condition, ...args) => {
      if (!condition) log('error')('Assertion failed', ...args);
    },
    time() {},
    timeEnd() {},
    group() {},
    groupEnd() {},
    clear() {},
  });

  // ── Timers, run by Rust between jobs ─────────────────────────────────────────────────────

  const timers = new Map();
  let lastTimer = 0;
  let idle = [];

  function arm(callback, delay, args, every) {
    if (typeof callback !== 'function') throw new TypeError('The callback must be a function');
    lastTimer += 1;
    const wait = Math.max(0, Number(delay) || 0);
    timers.set(lastTimer, { callback, args, at: Date.now() + wait, every: every ? wait : null });
    return lastTimer;
  }
  const disarm = (id) => {
    timers.delete(Number(id));
  };

  define(global, 'setTimeout', (callback, delay, ...args) => arm(callback, delay, args, false));
  define(global, 'setInterval', (callback, delay, ...args) => arm(callback, delay, args, true));
  define(global, 'setImmediate', (callback, ...args) => arm(callback, 0, args, false));
  define(global, 'clearTimeout', disarm);
  define(global, 'clearInterval', disarm);
  define(global, 'clearImmediate', disarm);
  if (typeof global.queueMicrotask !== 'function') {
    define(global, 'queueMicrotask', (callback) => {
      void Promise.resolve().then(callback);
    });
  }

  // Milliseconds to the next timer, `-1` for none.
  function nextTimer() {
    let soonest = -1;
    for (const timer of timers.values()) {
      if (soonest < 0 || timer.at < soonest) soonest = timer.at;
    }
    return soonest < 0 ? -1 : Math.max(0, soonest - Date.now());
  }

  function fireTimers() {
    const now = Date.now();
    for (const [id, timer] of [...timers]) {
      if (timer.at > now || !timers.has(id)) continue;
      if (timer.every === null) timers.delete(id);
      else timer.at = now + Math.max(1, timer.every);
      timer.callback(...timer.args);
    }
  }

  // Resolves once no timer is left: a script is over when what it scheduled has run.
  const settled = () =>
    timers.size === 0 ? Promise.resolve() : new Promise((resolve) => idle.push(resolve));

  function wakeIdle() {
    if (timers.size > 0 || idle.length === 0) return false;
    const waiting = idle;
    idle = [];
    for (const resolve of waiting) resolve();
    return true;
  }

  // ── Text, bytes and randomness ───────────────────────────────────────────────────────────

  function randomBytes(count) {
    const decoded = atobOf(native.random(count));
    const bytes = new Uint8Array(decoded.length);
    for (let index = 0; index < decoded.length; index += 1) bytes[index] = decoded.charCodeAt(index);
    return bytes;
  }

  define(global, 'crypto', {
    getRandomValues(array) {
      if (!ArrayBuffer.isView(array)) throw new TypeError('Expected a typed array');
      const bytes = randomBytes(array.byteLength);
      new Uint8Array(array.buffer, array.byteOffset, array.byteLength).set(bytes);
      return array;
    },
    randomUUID: () => uuid4(),
  });

  function uuid4() {
    const bytes = randomBytes(16);
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    const hex = [...bytes].map((byte) => byte.toString(16).padStart(2, '0')).join('');
    return `${hex.slice(0, 8)}-${hex.slice(8, 12)}-${hex.slice(12, 16)}-${hex.slice(16, 20)}-${hex.slice(20)}`;
  }

  const BASE64 = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/';

  function btoaOf(input) {
    const text = String(input);
    let out = '';
    for (let index = 0; index < text.length; index += 3) {
      const a = text.charCodeAt(index);
      const b = text.charCodeAt(index + 1);
      const c = text.charCodeAt(index + 2);
      if (a > 255 || b > 255 || c > 255) {
        throw new Error('btoa: the string holds a character outside Latin-1');
      }
      const triple = (a << 16) | ((b || 0) << 8) | (c || 0);
      out += BASE64[(triple >> 18) & 63] + BASE64[(triple >> 12) & 63];
      out += Number.isNaN(b) ? '=' : BASE64[(triple >> 6) & 63];
      out += Number.isNaN(c) ? '=' : BASE64[triple & 63];
    }
    return out;
  }

  function atobOf(input) {
    const text = String(input).replace(/[\s=]+/g, '');
    let out = '';
    let buffer = 0;
    let bits = 0;
    for (const character of text) {
      const value = BASE64.indexOf(character);
      if (value < 0) throw new Error('atob: the string is not Base64');
      buffer = (buffer << 6) | value;
      bits += 6;
      if (bits >= 8) {
        bits -= 8;
        out += String.fromCharCode((buffer >> bits) & 255);
      }
    }
    return out;
  }

  if (typeof global.btoa !== 'function') define(global, 'btoa', btoaOf);
  if (typeof global.atob !== 'function') define(global, 'atob', atobOf);

  if (typeof global.TextEncoder !== 'function') {
    define(
      global,
      'TextEncoder',
      class TextEncoder {
        get encoding() {
          return 'utf-8';
        }
        encode(input = '') {
          const binary = unescape(encodeURIComponent(String(input)));
          const bytes = new Uint8Array(binary.length);
          for (let index = 0; index < binary.length; index += 1) bytes[index] = binary.charCodeAt(index);
          return bytes;
        }
      },
    );
  }
  if (typeof global.TextDecoder !== 'function') {
    define(
      global,
      'TextDecoder',
      class TextDecoder {
        get encoding() {
          return 'utf-8';
        }
        decode(input) {
          if (input === undefined) return '';
          const bytes = ArrayBuffer.isView(input)
            ? new Uint8Array(input.buffer, input.byteOffset, input.byteLength)
            : new Uint8Array(input);
          let binary = '';
          for (const byte of bytes) binary += String.fromCharCode(byte);
          try {
            return decodeURIComponent(escape(binary));
          } catch {
            return binary;
          }
        }
      },
    );
  }

  define(global, 'process', {
    env: {},
    argv: [],
    browser: true,
    platform: 'browser',
    version: '',
    versions: {},
    cwd: () => '/',
    nextTick: (callback, ...args) => queueMicrotask(() => callback(...args)),
    emitWarning() {},
  });
  define(global, 'global', global);

  // ── require ──────────────────────────────────────────────────────────────────────────────

  const modules = new Map();
  const ALIASES = { 'csv-parse/lib/sync': 'csv-parse/sync', 'path-browserify': 'path' };
  const OWN = {
    atob: () => atobOf,
    btoa: () => btoaOf,
  };

  // Rust compiles a library, under its own name, as a function of these three.
  function load(name) {
    const factory = native.module(name);
    if (typeof factory !== 'function') return undefined;
    const module = { exports: {} };
    modules.set(name, module);
    factory(module, module.exports, requireOf);
    return module.exports;
  }

  function requireOf(requested) {
    const asked = String(requested)
      .replace(/^node:/, '')
      .replace(/\/$/, '');
    const name = ALIASES[asked] || asked;
    if (modules.has(name)) return modules.get(name).exports;
    if (OWN[name]) {
      const exports = OWN[name]();
      modules.set(name, { exports });
      return exports;
    }
    const exports = load(name);
    if (exports === undefined) {
      const error = new Error(`Cannot find module '${requested}'`);
      error.code = 'MODULE_NOT_FOUND';
      throw error;
    }
    // Postman's `csv-parse/lib/sync` is the function itself.
    if (asked === 'csv-parse/lib/sync') {
      const parse = exports.parse;
      Object.assign(parse, exports);
      modules.set(asked, { exports: parse });
      return parse;
    }
    return exports;
  }

  define(global, 'require', requireOf);
  lazy(global, 'Buffer', () => requireOf('buffer').Buffer);
  lazy(global, '_', () => requireOf('lodash'));
  lazy(global, 'CryptoJS', () => requireOf('crypto-js'));
  lazy(global, 'tv4', () => requireOf('tv4'));
  lazy(global, 'cheerio', () => requireOf('cheerio'));
  lazy(global, 'moment', () => requireOf('moment'));

  // ── Lists of key and value: headers, a query, a form ─────────────────────────────────────

  const sameKey = (a, b) => String(a).toLowerCase() === String(b).toLowerCase();

  function rowOf(item, value) {
    if (typeof item === 'string') {
      const colon = item.indexOf(':');
      if (value === undefined && colon > 0) {
        return { key: item.slice(0, colon).trim(), value: item.slice(colon + 1).trim(), disabled: false };
      }
      return { key: item, value: stored(value), disabled: false };
    }
    return {
      key: String(item.key ?? item.name ?? ''),
      value: stored(item.value),
      disabled: Boolean(item.disabled),
    };
  }

  class PropertyList {
    constructor(rows, caseless, changed) {
      this._rows = rows.map((row) => rowOf(row));
      this._caseless = caseless;
      this._changed = changed || (() => {});
    }
    _matches(key) {
      return (row) => (this._caseless ? sameKey(row.key, key) : row.key === String(key));
    }
    get(key) {
      const row = this._rows.find((candidate) => !candidate.disabled && this._matches(key)(candidate));
      return row === undefined ? undefined : row.value;
    }
    one(key) {
      return this._rows.find(this._matches(key));
    }
    has(key, value) {
      return this._rows.some(
        (row) =>
          !row.disabled && this._matches(key)(row) && (value === undefined || row.value === stored(value)),
      );
    }
    add(item, value) {
      this._rows.push(rowOf(item, value));
      this._changed();
    }
    append(item, value) {
      this.add(item, value);
    }
    prepend(item, value) {
      this._rows.unshift(rowOf(item, value));
      this._changed();
    }
    upsert(item, value) {
      const row = rowOf(item, value);
      const known = this._rows.find(this._matches(row.key));
      if (known === undefined) this._rows.push(row);
      else Object.assign(known, row);
      this._changed();
    }
    set(key, value) {
      this.upsert(key, value === undefined ? '' : value);
    }
    remove(predicate) {
      const drop =
        typeof predicate === 'function'
          ? predicate
          : typeof predicate === 'object' && predicate !== null
            ? this._matches(predicate.key)
            : this._matches(predicate);
      this._rows = this._rows.filter((row) => !drop(row));
      this._changed();
    }
    delete(key) {
      this.remove(key);
    }
    clear() {
      this._rows = [];
      this._changed();
    }
    populate(rows) {
      this._rows = rows.map((row) => rowOf(row));
      this._changed();
    }
    repopulate(rows) {
      this.populate(rows);
    }
    all() {
      return this._rows.map((row) => ({ ...row }));
    }
    count() {
      return this._rows.length;
    }
    idx(index) {
      return this._rows[index];
    }
    each(callback) {
      this._rows.forEach((row) => callback(row));
    }
    map(callback) {
      return this._rows.map((row) => callback(row));
    }
    filter(callback) {
      return this._rows.filter((row) => callback(row));
    }
    find(callback) {
      return this._rows.find((row) => callback(row));
    }
    reduce(callback, start) {
      return this._rows.reduce(callback, start);
    }
    toObject(excludeDisabled) {
      const out = {};
      for (const row of this._rows) {
        if (excludeDisabled && row.disabled) continue;
        out[row.key] = row.value;
      }
      return out;
    }
    toString() {
      return this._rows
        .filter((row) => !row.disabled)
        .map((row) => `${row.key}: ${row.value}`)
        .join('\n');
    }
    toJSON() {
      return this.all();
    }
    [Symbol.iterator]() {
      return this._rows[Symbol.iterator]();
    }
  }

  // ── A URL, read as typed: `{{baseUrl}}/users` is not a URL a parser accepts ──────────────

  const URL_PARTS =
    /^(?:([A-Za-z][A-Za-z0-9+.-]*):\/\/)?(?:([^@/?#]*)@)?([^/?#:]*)(?::([^/?#]*))?([^?#]*)(?:\?([^#]*))?(?:#(.*))?$/;

  function queryRows(text) {
    if (text === undefined || text === '') return [];
    return text.split('&').map((pair) => {
      const equals = pair.indexOf('=');
      return equals < 0
        ? { key: pair, value: null, disabled: false }
        : { key: pair.slice(0, equals), value: pair.slice(equals + 1), disabled: false };
    });
  }

  class Url {
    constructor(text) {
      this.update(text);
    }
    update(text) {
      const parts = URL_PARTS.exec(String(text ?? '').trim()) || [];
      this.protocol = parts[1];
      this.auth = parts[2];
      this.host = parts[3] ? parts[3].split('.') : [];
      this.port = parts[4];
      this.path = parts[5] ? parts[5].replace(/^\//, '').split('/') : [];
      this._trailing = parts[5] !== undefined && parts[5].endsWith('/') && parts[5] !== '/';
      this._rootOnly = parts[5] === '/';
      this.query = new PropertyList(queryRows(parts[6]), false);
      this.hash = parts[7];
    }
    getHost() {
      return this.host.join('.');
    }
    getRemote() {
      return this.port ? `${this.getHost()}:${this.port}` : this.getHost();
    }
    getPath() {
      if (this.path.length === 0 || (this.path.length === 1 && this.path[0] === '')) {
        return '/';
      }
      return `/${this.path.join('/')}`;
    }
    getQueryString() {
      return this.query._rows
        .filter((row) => !row.disabled)
        .map((row) => (row.value === null ? row.key : `${row.key}=${row.value}`))
        .join('&');
    }
    getPathWithQuery() {
      const query = this.getQueryString();
      return query ? `${this.getPath()}?${query}` : this.getPath();
    }
    addQueryParams(params) {
      for (const param of Array.isArray(params) ? params : queryRows(String(params))) this.query.add(param);
    }
    removeQueryParams(keys) {
      for (const key of [].concat(keys)) this.query.remove(typeof key === 'string' ? key : key.key);
    }
    toString() {
      let out = this.protocol ? `${this.protocol}://` : '';
      if (this.auth) out += `${this.auth}@`;
      out += this.getHost();
      if (this.port) out += `:${this.port}`;
      if (this.path.length > 0 && !(this.path.length === 1 && this.path[0] === '' && !this._rootOnly)) {
        out += `/${this.path.join('/')}`;
      }
      const query = this.getQueryString();
      if (query) out += `?${query}`;
      if (this.hash !== undefined) out += `#${this.hash}`;
      return out;
    }
    toJSON() {
      return this.toString();
    }
  }

  // ── A request ────────────────────────────────────────────────────────────────────────────

  class RequestBody {
    constructor(body) {
      const given = body || {};
      this.mode = given.mode || 'none';
      this.raw = given.raw ?? '';
      this.options = given.options || { raw: { language: given.language || 'text' } };
      this.urlencoded = new PropertyList(given.urlencoded || [], false);
      this.formdata = new PropertyList(given.formdata || [], false);
      this.file = given.file || { src: '' };
      this.graphql = given.graphql || { query: '', variables: '' };
    }
    update(next) {
      if (typeof next === 'string') {
        this.mode = 'raw';
        this.raw = next;
        return;
      }
      const body = new RequestBody(next);
      Object.assign(this, body);
    }
    isEmpty() {
      if (this.mode === 'raw') return this.raw === '';
      if (this.mode === 'urlencoded') return this.urlencoded.count() === 0;
      if (this.mode === 'formdata') return this.formdata.count() === 0;
      if (this.mode === 'file') return !this.file.src;
      return this.mode === 'none';
    }
    toString() {
      if (this.mode === 'raw') return this.raw;
      if (this.mode === 'urlencoded') {
        return this.urlencoded._rows
          .filter((row) => !row.disabled)
          .map((row) => `${encodeURIComponent(row.key)}=${encodeURIComponent(row.value)}`)
          .join('&');
      }
      return '';
    }
    toJSON() {
      return {
        mode: this.mode,
        raw: this.raw,
        language: (this.options && this.options.raw && this.options.raw.language) || 'text',
        urlencoded: this.urlencoded.all(),
        formdata: this.formdata._rows.map((row) => ({
          key: row.key,
          value: row.value,
          disabled: row.disabled,
          type: row.type === 'file' ? 'file' : 'text',
        })),
        file: this.file,
        graphql: this.graphql,
      };
    }
  }

  class Request {
    constructor(given) {
      this.method = String(given.method || 'GET').toUpperCase();
      this.url = given.url instanceof Url ? given.url : new Url(given.url);
      this.headers = new PropertyList(headerRows(given.header ?? given.headers), true);
      this.body = given.body instanceof RequestBody ? given.body : new RequestBody(given.body);
      this.name = given.name;
      this.id = given.id;
    }
    addHeader(header) {
      this.headers.add(header);
    }
    removeHeader(name) {
      this.headers.remove(name);
    }
    upsertHeader(header) {
      this.headers.upsert(header);
    }
    getHeaders(options) {
      const out = {};
      for (const row of this.headers._rows) {
        if (options && options.enabled && row.disabled) continue;
        out[options && options.ignoreCase ? row.key.toLowerCase() : row.key] = row.value;
      }
      return out;
    }
    toJSON() {
      return {
        method: this.method,
        url: this.url.toString(),
        headers: this.headers.all(),
        body: this.body.toJSON(),
      };
    }
  }

  function headerRows(given) {
    if (given === undefined || given === null) return [];
    if (given instanceof PropertyList) return given.all();
    if (Array.isArray(given)) return given;
    if (typeof given === 'string') {
      return given
        .split(/\r?\n/)
        .filter((text) => text.includes(':'))
        .map((text) => rowOf(text));
    }
    return Object.entries(given).map(([key, value]) => ({ key, value: stored(value), disabled: false }));
  }

  // `req`: the request in four words, over the same object `pm.request` edits.
  function plainRequest(request) {
    return {
      get method() {
        return request.method;
      },
      set method(value) {
        request.method = String(value).toUpperCase();
      },
      get url() {
        return request.url.toString();
      },
      set url(value) {
        request.url.update(value);
      },
      get headers() {
        return request.headers;
      },
      get body() {
        return request.body.toString();
      },
      set body(value) {
        request.body.update(typeof value === 'string' ? value : stored(value));
      },
      json() {
        return JSON.parse(request.body.toString());
      },
    };
  }

  // ── A response ───────────────────────────────────────────────────────────────────────────

  class CookieList {
    constructor(cookies) {
      this._cookies = cookies || [];
    }
    get(name) {
      const cookie = this._cookies.find((candidate) => candidate.name === name);
      return cookie === undefined ? undefined : cookie.value;
    }
    one(name) {
      return this._cookies.find((candidate) => candidate.name === name);
    }
    has(name) {
      return this._cookies.some((candidate) => candidate.name === name);
    }
    all() {
      return this._cookies.map((cookie) => ({ ...cookie }));
    }
    count() {
      return this._cookies.length;
    }
    each(callback) {
      this._cookies.forEach((cookie) => callback(cookie));
    }
    toObject() {
      return Object.fromEntries(this._cookies.map((cookie) => [cookie.name, cookie.value]));
    }
    jar() {
      return jar;
    }
  }

  class Response {
    constructor(given) {
      this.code = given.code;
      this.status = given.reason || '';
      this.headers = new PropertyList(given.headers || [], true);
      this.responseTime = given.millis;
      this.responseSize = given.size;
      this.cookies = new CookieList(given.cookies);
      this._body = given.body || '';
    }
    text() {
      return this._body;
    }
    json() {
      try {
        return JSON.parse(this._body);
      } catch (error) {
        throw new SyntaxError(`The response is not JSON: ${error.message}`, { cause: error });
      }
    }
    reason() {
      return this.status;
    }
    size() {
      return { body: this.responseSize, header: 0, total: this.responseSize };
    }
    get stream() {
      return requireOf('buffer').Buffer.from(this._body);
    }
    get to() {
      return expectOf(this).to;
    }
    get not() {
      return expectOf(this).not;
    }
    toJSON() {
      return { code: this.code, status: this.status, headers: this.headers.all(), body: this._body };
    }
  }

  // `res`: the response in plain values, its headers an object by their lowercased name.
  function plainResponse(response) {
    const headers = {};
    for (const row of response.headers._rows) {
      const key = row.key.toLowerCase();
      headers[key] = key in headers ? `${headers[key]}, ${row.value}` : row.value;
    }
    return {
      status: response.code,
      statusText: response.status,
      headers,
      body: response.text(),
      time: response.responseTime,
      size: response.responseSize,
      cookies: response.cookies.toObject(),
      text: () => response.text(),
      json: () => response.json(),
    };
  }

  // ── Variables, the nearest level first ───────────────────────────────────────────────────

  const DYNAMIC = {
    $guid: () => uuid4(),
    $randomUUID: () => uuid4(),
    $timestamp: () => String(Math.floor(Date.now() / 1000)),
    $isoTimestamp: () => new Date().toISOString(),
    $randomInt: () => {
      const [high, low] = randomBytes(2);
      return String(((high << 8) | low) % 1001);
    },
  };

  function resolve(name) {
    if (state.local.has(name)) return state.local.get(name);
    for (const layer of state.layers) {
      if (Object.prototype.hasOwnProperty.call(layer.values, name)) return layer.values[name];
    }
    return undefined;
  }

  function replaceIn(text) {
    if (typeof text !== 'string') return text;
    return text.replace(/\{\{(\$?[A-Za-z0-9_-]+)\}\}/g, (whole, name) => {
      if (DYNAMIC[name]) return DYNAMIC[name]();
      const value = resolve(name);
      return value === undefined ? whole : value;
    });
  }

  function layerOf(scope) {
    return state.layers.find((layer) => layer.scope === scope);
  }

  function write(scope, name, value, secret) {
    const key = String(name);
    const layer = layerOf(scope);
    if (layer !== undefined) {
      if (value === undefined) delete layer.values[key];
      else layer.values[key] = value;
    }
    state.writes.push({
      scope,
      name: key,
      value: value === undefined ? null : value,
      secret: Boolean(secret),
    });
  }

  // Where `env.set` and `pm.environment` write: the active environment, else the globals.
  const environmentScope = () => (state.job && state.job.environment !== null ? 'environment' : 'globals');

  class VariableScope {
    constructor(scope, name) {
      this._scope = typeof scope === 'function' ? scope : () => scope;
      this.name = name;
    }
    _values() {
      const layer = layerOf(this._scope());
      return layer === undefined ? {} : layer.values;
    }
    get(key) {
      return this._values()[key];
    }
    has(key) {
      return Object.prototype.hasOwnProperty.call(this._values(), key);
    }
    set(key, value) {
      write(this._scope(), key, stored(value), false);
    }
    unset(key) {
      write(this._scope(), key, undefined, false);
    }
    clear() {
      for (const key of Object.keys(this._values())) this.unset(key);
    }
    toObject() {
      return { ...this._values() };
    }
    replaceIn(text) {
      return replaceIn(text);
    }
    get values() {
      return new PropertyList(
        Object.entries(this._values()).map(([key, value]) => ({ key, value })),
        false,
      );
    }
  }

  const localScope = {
    get: (key) => resolve(key),
    has: (key) => resolve(key) !== undefined,
    set: (key, value) => {
      state.local.set(String(key), stored(value));
    },
    unset: (key) => {
      state.local.delete(String(key));
    },
    clear: () => state.local.clear(),
    toObject: () => {
      const out = {};
      for (const layer of [...state.layers].reverse()) Object.assign(out, layer.values);
      for (const [key, value] of state.local) out[key] = value;
      return out;
    },
    replaceIn,
  };

  const env = {
    get: (name) => resolve(String(name)),
    set: (name, value) => write(environmentScope(), name, stored(value), false),
    secret: (name, value) => write(environmentScope(), name, stored(value), true),
    unset: (name) => write(environmentScope(), name, undefined, false),
    has: (name) => resolve(String(name)) !== undefined,
    replaceIn,
  };

  // ── Tests ────────────────────────────────────────────────────────────────────────────────

  function failureOf(name, error) {
    const place = placeOf(error);
    const assertion = error && typeof error === 'object' && 'actual' in error;
    return {
      name,
      passed: false,
      skipped: false,
      message: error && error.message !== undefined ? String(error.message) : String(error),
      expected: assertion && 'expected' in error ? show(error.expected) : null,
      actual: assertion ? show(error.actual) : null,
      script: place.script || (state.script && state.script.id),
      line: place.line,
    };
  }

  function test(name, run) {
    const title = String(name);
    const result = {
      name: title,
      passed: true,
      skipped: false,
      message: null,
      expected: null,
      actual: null,
      script: state.script && state.script.id,
      line: null,
    };
    const at = state.tests.push(result) - 1;
    try {
      const answer =
        typeof run === 'function' ? (run.length > 0 ? new Promise((done) => run(done)) : run()) : undefined;
      if (answer && typeof answer.then === 'function') {
        state.pending.push(
          Promise.resolve(answer).then(
            () => {},
            (error) => {
              state.tests[at] = failureOf(title, error);
            },
          ),
        );
      }
    } catch (error) {
      state.tests[at] = failureOf(title, error);
    }
  }
  test.skip = (name) => {
    state.tests.push({
      name: String(name),
      passed: false,
      skipped: true,
      message: null,
      expected: null,
      actual: null,
      script: state.script && state.script.id,
      line: null,
    });
  };

  // ── expect: chai's, with Jest's words and Postman's response assertions ──────────────────

  let chai = null;

  function chaiOf() {
    if (chai !== null) return chai;
    chai = requireOf('chai');
    chai.config.truncateThreshold = 200;
    chai.use(jestWords);
    chai.use(responseWords);
    return chai;
  }

  const expectOf = (value, message) => chaiOf().expect(value, message);

  function at(object, path) {
    const keys = Array.isArray(path)
      ? path
      : String(path)
          .replace(/\[(\w+)\]/g, '.$1')
          .split('.')
          .filter(Boolean);
    let current = object;
    for (const key of keys) {
      if (
        current === null ||
        current === undefined ||
        !(Object(current) === current || typeof current === 'string')
      ) {
        return { found: false };
      }
      if (!(key in Object(current))) return { found: false };
      current = current[key];
    }
    return { found: true, value: current };
  }

  function subset(actual, expected, eql) {
    if (expected === null || typeof expected !== 'object') return eql(actual, expected);
    if (actual === null || typeof actual !== 'object') return false;
    if (Array.isArray(expected)) {
      return (
        Array.isArray(actual) &&
        actual.length === expected.length &&
        expected.every((item, index) => subset(actual[index], item, eql))
      );
    }
    return Object.keys(expected).every((key) => key in actual && subset(actual[key], expected[key], eql));
  }

  function jestWords(lib, util) {
    const { Assertion } = lib;
    const eql = util.eql;
    const add = (name, assert) => Assertion.addMethod(name, assert);
    add('toBe', function (expected) {
      this.assert(
        Object.is(this._obj, expected),
        'expected #{this} to be #{exp}',
        'expected #{this} not to be #{exp}',
        expected,
        this._obj,
      );
    });
    add('toEqual', function (expected) {
      this.assert(
        eql(this._obj, expected),
        'expected #{this} to equal #{exp}',
        'expected #{this} not to equal #{exp}',
        expected,
        this._obj,
        true,
      );
    });
    add('toStrictEqual', function (expected) {
      this.assert(
        eql(this._obj, expected),
        'expected #{this} to strictly equal #{exp}',
        'expected #{this} not to strictly equal #{exp}',
        expected,
        this._obj,
        true,
      );
    });
    add('toContain', function (item) {
      const target = this._obj;
      const found =
        typeof target === 'string'
          ? target.includes(item)
          : target !== null && target !== undefined && [...target].some((value) => value === item);
      this.assert(
        found,
        'expected #{this} to contain #{exp}',
        'expected #{this} not to contain #{exp}',
        item,
        target,
      );
    });
    add('toContainEqual', function (item) {
      const found = [...this._obj].some((value) => eql(value, item));
      this.assert(
        found,
        'expected #{this} to contain an item equal to #{exp}',
        'expected #{this} not to contain an item equal to #{exp}',
        item,
        this._obj,
      );
    });
    add('toMatch', function (pattern) {
      const text = String(this._obj);
      const found = pattern instanceof RegExp ? pattern.test(text) : text.includes(String(pattern));
      this.assert(
        found,
        'expected #{this} to match #{exp}',
        'expected #{this} not to match #{exp}',
        pattern,
        this._obj,
      );
    });
    add('toMatchObject', function (expected) {
      this.assert(
        subset(this._obj, expected, eql),
        'expected #{this} to match the object #{exp}',
        'expected #{this} not to match the object #{exp}',
        expected,
        this._obj,
        true,
      );
    });
    const compare = (name, test, words) =>
      add(name, function (bound) {
        this.assert(
          test(this._obj, bound),
          `expected #{this} to be ${words} #{exp}`,
          `expected #{this} not to be ${words} #{exp}`,
          bound,
          this._obj,
        );
      });
    compare('toBeLessThan', (a, b) => a < b, 'less than');
    compare('toBeLessThanOrEqual', (a, b) => a <= b, 'less than or equal to');
    compare('toBeGreaterThan', (a, b) => a > b, 'greater than');
    compare('toBeGreaterThanOrEqual', (a, b) => a >= b, 'greater than or equal to');
    add('toBeCloseTo', function (expected, digits = 2) {
      const near = Math.abs(expected - this._obj) < Math.pow(10, -digits) / 2;
      this.assert(
        near,
        'expected #{this} to be close to #{exp}',
        'expected #{this} not to be close to #{exp}',
        expected,
        this._obj,
      );
    });
    add('toHaveProperty', function (path, ...value) {
      const found = at(this._obj, path);
      const holds = found.found && (value.length === 0 || eql(found.value, value[0]));
      this.assert(
        holds,
        `expected #{this} to have the property ${show(path)}${value.length ? ' of #{exp}' : ''}`,
        `expected #{this} not to have the property ${show(path)}${value.length ? ' of #{exp}' : ''}`,
        value.length ? value[0] : path,
        found.found ? found.value : undefined,
      );
    });
    add('toHaveLength', function (length) {
      const actual = this._obj === null || this._obj === undefined ? undefined : this._obj.length;
      this.assert(
        actual === length,
        'expected #{this} to have a length of #{exp}',
        'expected #{this} not to have a length of #{exp}',
        length,
        actual,
      );
    });
    const state1 = (name, test, words) =>
      add(name, function () {
        this.assert(
          test(this._obj),
          `expected #{this} to be ${words}`,
          `expected #{this} not to be ${words}`,
          words,
          this._obj,
        );
      });
    state1('toBeTruthy', (value) => Boolean(value), 'truthy');
    state1('toBeFalsy', (value) => !value, 'falsy');
    state1('toBeNull', (value) => value === null, 'null');
    state1('toBeUndefined', (value) => value === undefined, 'undefined');
    state1('toBeDefined', (value) => value !== undefined, 'defined');
    state1('toBeNaN', (value) => Number.isNaN(value), 'NaN');
    add('toBeInstanceOf', function (type) {
      this.assert(
        this._obj instanceof type,
        `expected #{this} to be an instance of ${type && type.name}`,
        `expected #{this} not to be an instance of ${type && type.name}`,
        type && type.name,
        this._obj,
      );
    });
    add('toThrow', function (expected) {
      let thrown = null;
      let threw = false;
      try {
        this._obj();
      } catch (error) {
        threw = true;
        thrown = error;
      }
      const message = thrown && thrown.message !== undefined ? String(thrown.message) : String(thrown);
      const fits =
        threw &&
        (expected === undefined ||
          (expected instanceof RegExp && expected.test(message)) ||
          (typeof expected === 'string' && message.includes(expected)) ||
          (typeof expected === 'function' && thrown instanceof expected));
      this.assert(
        fits,
        'expected the function to throw #{exp}',
        'expected the function not to throw #{exp}',
        expected === undefined ? 'an error' : expected,
        threw ? message : 'nothing',
      );
    });
  }

  const isResponse = (value) => value instanceof Response;

  function responseWords(lib, util) {
    const { Assertion } = lib;
    const flag = util.flag;
    const own = (name, read, kind) =>
      (kind === 'method' ? Assertion.overwriteMethod : Assertion.overwriteProperty).call(
        Assertion,
        name,
        (base) =>
          function (...args) {
            if (!isResponse(flag(this, 'object'))) return base.apply(this, args);
            return read.apply(this, args);
          },
      );
    const classes = {
      info: [100, 199],
      success: [200, 299],
      redirection: [300, 399],
      clientError: [400, 499],
      serverError: [500, 599],
      error: [400, 599],
    };
    for (const [name, [low, high]] of Object.entries(classes)) {
      Assertion.addProperty(name, function () {
        const code = flag(this, 'object').code;
        this.assert(
          code >= low && code <= high,
          `expected the response's status ${code} to be ${name}`,
          `expected the response's status ${code} not to be ${name}`,
          `${low}-${high}`,
          code,
        );
      });
    }
    const codes = {
      accepted: 202,
      badRequest: 400,
      unauthorized: 401,
      forbidden: 403,
      notFound: 404,
      rateLimited: 429,
    };
    for (const [name, wanted] of Object.entries(codes)) {
      Assertion.addProperty(name, function () {
        const code = flag(this, 'object').code;
        this.assert(
          code === wanted,
          `expected the response's status ${code} to be ${wanted}`,
          `expected the response's status ${code} not to be ${wanted}`,
          wanted,
          code,
        );
      });
    }
    own('ok', function () {
      const code = flag(this, 'object').code;
      this.assert(
        code === 200,
        `expected the response's status ${code} to be 200`,
        `expected the response's status ${code} not to be 200`,
        200,
        code,
      );
    });
    Assertion.addMethod('status', function (wanted) {
      const response = flag(this, 'object');
      const actual = typeof wanted === 'number' ? response.code : response.status;
      this.assert(
        actual === wanted,
        'expected the response to have the status #{exp} but got #{act}',
        'expected the response not to have the status #{exp}',
        wanted,
        actual,
      );
    });
    Assertion.addMethod('header', function (name, ...value) {
      const response = flag(this, 'object');
      const actual = response.headers.get(name);
      const holds = actual !== undefined && (value.length === 0 || actual === stored(value[0]));
      this.assert(
        holds,
        `expected the response to have the header ${show(name)}${value.length ? ' of #{exp}' : ''}`,
        `expected the response not to have the header ${show(name)}${value.length ? ' of #{exp}' : ''}`,
        value.length ? value[0] : name,
        actual,
      );
    });
    Assertion.addMethod('body', function (...value) {
      const response = flag(this, 'object');
      const text = response.text();
      let holds = text !== '';
      if (value.length > 0) {
        holds =
          typeof value[0] === 'string'
            ? text === value[0]
            : value[0] instanceof RegExp
              ? value[0].test(text)
              : util.eql(safeJson(text), value[0]);
      }
      this.assert(
        holds,
        'expected the response to have the body #{exp}',
        'expected the response not to have the body #{exp}',
        value.length ? value[0] : 'a body',
        text,
      );
    });
    Assertion.addMethod('jsonBody', function (...args) {
      const response = flag(this, 'object');
      const parsed = safeJson(response.text());
      let holds = parsed !== undefined;
      let actual = parsed;
      if (holds && args.length > 0) {
        if (typeof args[0] === 'string') {
          const found = at(parsed, args[0]);
          actual = found.value;
          holds = found.found && (args.length < 2 || util.eql(found.value, args[1]));
        } else {
          holds = util.eql(parsed, args[0]);
        }
      }
      this.assert(
        holds,
        'expected the response to have the JSON body #{exp}',
        'expected the response not to have the JSON body #{exp}',
        args.length > 1 ? args[1] : args[0],
        actual,
      );
    });
    Assertion.addProperty('json', function () {
      this.assert(
        safeJson(flag(this, 'object').text()) !== undefined,
        'expected the response to be JSON',
        'expected the response not to be JSON',
      );
    });
    Assertion.addProperty('withBody', function () {
      this.assert(
        flag(this, 'object').text() !== '',
        'expected the response to have a body',
        'expected the response to have no body',
      );
    });
    Assertion.addMethod('responseTime', function (bound) {
      const time = flag(this, 'object').responseTime;
      this.assert(
        time <= bound,
        'expected the response to arrive within #{exp} ms but it took #{act}',
        'expected the response to take longer than #{exp} ms',
        bound,
        time,
      );
    });
  }

  function safeJson(text) {
    try {
      return JSON.parse(text);
    } catch {
      return undefined;
    }
  }

  lazy(global, 'expect', () => (value, message) => expectOf(value, message));

  // ── Talking to Rust: a send, the jar ─────────────────────────────────────────────────────

  function call(kind, payload) {
    const answer = JSON.parse(native.call(kind, JSON.stringify(payload)));
    if (answer.error !== undefined) {
      const error = new Error(answer.error);
      error.code = answer.code;
      throw error;
    }
    return answer.ok;
  }

  function later(callback, ...args) {
    if (typeof callback === 'function') queueMicrotask(() => callback(...args));
  }

  function sendRequest(given, callback) {
    const request = new Request(typeof given === 'string' ? { url: given } : given || {});
    let outcome;
    try {
      const response = new Response(call('send', request.toJSON()));
      outcome = { response };
    } catch (error) {
      outcome = { error };
    }
    if (typeof callback === 'function') {
      later(callback, outcome.error || null, outcome.response);
      return undefined;
    }
    return outcome.error ? Promise.reject(outcome.error) : Promise.resolve(outcome.response);
  }

  const jarCall = (kind, payload, callback) => {
    try {
      later(callback, null, call(kind, payload));
    } catch (error) {
      later(callback, error);
    }
  };
  const jar = {
    get: (url, name, callback) => jarCall('jarGet', { url: String(url), name: String(name) }, callback),
    getAll: (url, options, callback) =>
      jarCall('jarAll', { url: String(url) }, typeof options === 'function' ? options : callback),
    set: (url, name, value, callback) => {
      const cookie = typeof name === 'object' ? name : { name, value };
      jarCall(
        'jarSet',
        {
          url: String(url),
          cookie: {
            name: String(cookie.name),
            value: stored(cookie.value),
            path: cookie.path,
            domain: cookie.domain,
            secure: cookie.secure,
            httpOnly: cookie.httpOnly,
            expires: cookie.expires,
          },
        },
        typeof value === 'function' ? value : callback,
      );
    },
    unset: (url, name, callback) => jarCall('jarUnset', { url: String(url), name: String(name) }, callback),
    clear: (url, callback) => jarCall('jarClear', { url: String(url) }, callback),
  };

  // ── pm, and Postman's older names ────────────────────────────────────────────────────────

  function install(job) {
    const request = state.request;
    const response = state.response;
    const environment = new VariableScope(
      environmentScope,
      job.environment === null ? undefined : job.environment,
    );
    const pm = {
      info: {
        eventName: job.phase === 'before' ? 'prerequest' : 'test',
        iteration: job.info.iteration,
        iterationCount: job.info.iterationCount,
        requestName: job.info.requestName,
        requestId: job.info.requestId,
      },
      request,
      response,
      environment,
      globals: new VariableScope('globals'),
      collectionVariables: new VariableScope('collection'),
      variables: localScope,
      iterationData: new VariableScope('data'),
      cookies: new CookieList(job.cookies),
      test,
      expect: (value, message) => expectOf(value, message),
      sendRequest,
      execution: {
        skipRequest() {
          if (job.phase !== 'before')
            throw new Error('pm.execution.skipRequest() works only before the request is sent');
          state.skip = true;
        },
        setNextRequest(name) {
          state.next = name === null ? null : String(name);
        },
        location: job.scripts.map((script) => script.name),
      },
      visualizer: {
        set() {
          console.warn('pm.visualizer is not supported: nothing is drawn');
        },
        clear() {},
      },
      vault: {
        get: async () => undefined,
        set: async () => undefined,
        unset: async () => undefined,
      },
      require() {
        throw new Error("pm.require reads Postman's package library, which is not available here");
      },
    };
    define(global, 'pm', pm);
    define(global, 'env', env);
    define(global, 'test', test);
    define(global, 'req', plainRequest(request));
    define(global, 'res', response === null ? undefined : plainResponse(response));

    const tests = {};
    define(global, 'tests', tests);
    define(global, 'postman', {
      setEnvironmentVariable: (key, value) => environment.set(key, value),
      getEnvironmentVariable: (key) => environment.get(key),
      clearEnvironmentVariable: (key) => environment.unset(key),
      clearEnvironmentVariables: () => environment.clear(),
      setGlobalVariable: (key, value) => pm.globals.set(key, value),
      getGlobalVariable: (key) => pm.globals.get(key),
      clearGlobalVariable: (key) => pm.globals.unset(key),
      clearGlobalVariables: () => pm.globals.clear(),
      setNextRequest: (name) => pm.execution.setNextRequest(name),
      getResponseHeader: (name) => (response === null ? undefined : response.headers.get(name)),
      getResponseCookie: (name) => (response === null ? undefined : response.cookies.one(name)),
    });
    define(global, 'environment', environment.toObject());
    define(global, 'globals', pm.globals.toObject());
    define(global, 'data', pm.iterationData.toObject());
    define(global, 'iteration', job.info.iteration);
    define(global, 'request', {
      url: request.url.toString(),
      method: request.method,
      headers: request.getHeaders(),
      data: request.body.toString(),
    });
    if (response !== null) {
      define(global, 'responseBody', response.text());
      define(global, 'responseCode', { code: response.code, name: response.status, detail: response.status });
      define(global, 'responseTime', response.responseTime);
      define(global, 'responseHeaders', response.headers.toObject());
      define(global, 'responseCookies', response.cookies.all());
    }
    define(global, 'xml2Json', (xml) => {
      let parsed;
      requireOf('xml2js').parseString(xml, { explicitArray: false, async: false }, (error, result) => {
        if (error) throw error;
        parsed = result;
      });
      return parsed;
    });
    return tests;
  }

  // ── A phase: every script in turn, then what it left to Rust ─────────────────────────────

  async function phase(job, functions) {
    state.job = job;
    state.writes = [];
    state.tests = [];
    state.skip = false;
    state.next = undefined;
    state.layers = job.layers.map((layer) => ({
      scope: layer.scope,
      from: layer.from,
      values: { ...layer.values },
    }));
    state.request = new Request(job.request);
    state.response = job.response === null ? null : new Response(job.response);
    const legacy = install(job);
    let failure = null;
    for (let index = 0; index < functions.length; index += 1) {
      state.script = job.scripts[index];
      state.pending = [];
      try {
        await functions[index]();
        await Promise.all(state.pending);
        await settled();
      } catch (error) {
        const place = placeOf(error);
        failure = {
          script: place.script || state.script.id,
          line: place.line,
          message:
            error && error.message !== undefined
              ? `${error.name || 'Error'}: ${error.message}`
              : String(error),
        };
        break;
      }
    }
    for (const [name, passed] of Object.entries(legacy)) {
      state.tests.push({
        name,
        passed: Boolean(passed),
        skipped: false,
        message: passed ? null : 'false',
        expected: null,
        actual: null,
        script: null,
        line: null,
      });
    }
    state.script = null;
    return JSON.stringify({
      request: job.phase === 'before' ? state.request.toJSON() : null,
      writes: state.writes,
      tests: state.tests,
      failure,
      skip: state.skip,
      locals: Object.fromEntries(state.local),
      next: state.next === undefined ? { set: false, name: null } : { set: true, name: state.next },
    });
  }

  define(global, '__devnotes', { phase, nextTimer, fireTimers, wakeIdle });
})(globalThis.__native);
