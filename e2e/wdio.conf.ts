import { existsSync, mkdirSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import { EMBEDDED_PORT } from './support/leftovers.js';
import { SCRIPT_TIMEOUT } from './support/timeouts.js';

const here = dirname(fileURLToPath(import.meta.url));
const repo = resolve(here, '..');

/**
 * Built by `npm run e2e:build`, which links the `e2e` Cargo feature and merges
 * `tauri.e2e.conf.json`. A release binary carries neither and cannot be driven.
 * `CARGO_TARGET_DIR` is read as cargo reads it under the Tauri CLI, from `src-tauri/`.
 */
const appBinary = resolve(
  repo,
  'src-tauri',
  process.env['CARGO_TARGET_DIR'] ?? 'target',
  'debug',
  process.platform === 'win32' ? 'devnotes.exe' : 'devnotes',
);

/**
 * `E2E_SPEC=05-spaces` runs that file alone, which is how CI runs each scenario of a group in
 * turn. A name that matches no file fails here: a run that ran nothing would pass.
 */
function specs(): string[] {
  const name = process.env['E2E_SPEC'];
  if (!name) return ['./specs/**/*.e2e.ts'];

  const file = join(here, 'specs', name.endsWith('.e2e.ts') ? name : `${name}.e2e.ts`);
  if (!existsSync(file)) {
    throw new Error(`E2E_SPEC=${name} names no scenario: ${file} does not exist`);
  }
  return [file];
}

/**
 * The WebDriver server runs inside the application, so there is no `tauri-driver`, no
 * msedgedriver and no Chrome DevTools protocol — nor anything those need from their host.
 *
 * ⚠️ The cost shapes every spec file: this provider spawns the application once, from its
 * own `onPrepare`. Every spec file of a run drives the same process, the same SQLite file
 * and the same `preferences.json`, and bumping `maxInstances` does not change it. So the
 * profile is wiped before the runner starts, `before()` buys a fresh front end and nothing
 * more, and a spec file establishes its own preconditions. See `docs/architecture.md`.
 */
const driverProvider = 'embedded';

/**
 * The application's stdout, into a file under `outputDir`: Rust logs through the log plugin's
 * stdout target. ⚠️ Not the WebView's console: the polyfill hands it to `window.__TAURI__.log`
 * first, and allowing that plugin command fed its own failures back into the console, hundreds
 * of lines a file. `beforeTest` records the console in the page instead.
 */
const serviceOptions = {
  driverProvider,
  appBinaryPath: appBinary,
  captureBackendLogs: true,
  // From spawn to a listening WebDriver server. A fresh Windows runner took 18 s on average,
  // 34 s once, and past the default 60 s once: the first launch of a downloaded binary.
  startTimeout: 180_000,
} as const;

/** `E2E_LOG_LEVEL=trace` when a session refuses to open; the default keeps runs readable. */
const logLevel = (process.env['E2E_LOG_LEVEL'] ?? 'warn') as NonNullable<WebdriverIO.Config['logLevel']>;

/** The refresh below undoes what a previous file left behind, and there is none yet. */
let aFileHasRun = false;

export const config: WebdriverIO.Config = {
  runner: 'local',
  specs: specs(),

  /**
   * One worker: the files of a run share the application, and the run order is the
   * numeric prefix. Only `01-first-launch` needs a place in it — first, on a virgin
   * profile; every other file runs alone too, as it does on CI.
   */
  maxInstances: 1,

  // The first failure ends the file and the run (`mochaOpts.bail` the file): what follows it
  // fails in cascade, and buries the one error worth reading.
  bail: 1,

  capabilities: [
    {
      browserName: 'tauri',
      'wdio:enforceWebDriverClassic': true,
      'tauri:options': { application: appBinary },
      'wdio:tauriServiceOptions': serviceOptions,
    },
  ] as unknown as WebdriverIO.Capabilities[],

  // `embeddedPort` is the base — each worker gets it plus its index.
  services: [['@wdio/tauri-service', { ...serviceOptions, embeddedPort: EMBEDDED_PORT }]],

  framework: 'mocha',
  reporters: ['spec'],
  mochaOpts: { ui: 'bdd', timeout: 120_000, bail: true },

  logLevel,
  outputDir: join(here, 'logs'),
  waitforTimeout: 10_000,
  connectionRetryTimeout: 120_000,
  connectionRetryCount: 3,

  /**
   * One application serves the whole run, so a spec file inherits whatever the previous
   * one left on screen. Reloading gives each file a fresh front end over the shared
   * database. Imported inside the hook: the launcher loads this file too, with no
   * `browser` to speak of.
   */
  async before() {
    const { browser } = await import('@wdio/globals');
    await browser.setTimeout({ script: SCRIPT_TIMEOUT });

    // Never before the first file: there is nothing to clear, and the application is
    // still seeding its samples. `SampleNotesService` writes its marker before the notes,
    // so a front end reloaded inside that window skips the seeding it interrupted — and
    // every launch after it agrees there is nothing to do.
    if (!aFileHasRun) {
      aFileHasRun = true;
      return;
    }

    await browser.refresh();
  },

  /**
   * The page's warnings and errors, kept for `afterTest`. A reload empties them, and the next
   * test installs the recorder again. A page caught mid-reload goes without: a diagnostic never
   * fails a test.
   */
  async beforeTest() {
    const { browser } = await import('@wdio/globals');

    await browser
      .execute(() => {
        const page = window as unknown as { __e2eConsole?: string[] };
        if (page.__e2eConsole) return;

        const seen: string[] = [];
        page.__e2eConsole = seen;
        const text = (value: unknown): string => {
          if (typeof value === 'string') return value;
          if (value instanceof Error) return value.stack ?? value.message;
          try {
            return JSON.stringify(value);
          } catch {
            return String(value);
          }
        };

        for (const level of ['warn', 'error'] as const) {
          const original = console[level].bind(console);
          console[level] = (...args: unknown[]) => {
            seen.push(`${level}: ${args.map(text).join(' ')}`.slice(0, 2_000));
            if (seen.length > 200) seen.shift();
            original(...args);
          };
        }
      })
      .catch(() => undefined);
  },

  /**
   * "The row is missing" and "the field was never filled" read exactly the same from an
   * assertion message. This writes down what was actually on screen.
   */
  async afterTest(test, _context, result) {
    if (result.passed) return;

    const { browser } = await import('@wdio/globals');
    const directory = join(here, 'logs', 'failures');
    mkdirSync(directory, { recursive: true });

    const name = `${test.parent} ${test.title}`.replace(/[^a-z0-9]+/gi, '-').slice(0, 110);

    try {
      await browser.saveScreenshot(join(directory, `${name}.png`));

      const state = await browser.execute(() => ({
        cards: [...document.querySelectorAll('[data-testid="note-card-title"]')].map((card) =>
          (card.textContent ?? '').trim(),
        ),
        busy: document.querySelector('[data-testid="canvas"]')?.getAttribute('aria-busy') ?? null,
        // A value that never arrived looks nothing like a write that was lost.
        fields: [...document.querySelectorAll('input, textarea')].map((field) => ({
          testid: field.getAttribute('data-testid'),
          value: (field as HTMLInputElement).value,
        })),
        dialogs: [...document.querySelectorAll('[role="dialog"]')].map(
          (dialog) => dialog.getAttribute('aria-labelledby') ?? dialog.getAttribute('aria-label'),
        ),
        error: document.querySelector('[data-testid="error-banner"]')?.textContent?.trim() ?? null,
        console: (window as unknown as { __e2eConsole?: string[] }).__e2eConsole ?? [],
      }));

      writeFileSync(join(directory, `${name}.json`), JSON.stringify(state, null, 2), 'utf8');
    } catch (error) {
      writeFileSync(join(directory, `${name}.json`), `capture failed: ${String(error)}`, 'utf8');
    }
  },
};
