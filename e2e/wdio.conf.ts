import { mkdirSync, writeFileSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const here = dirname(fileURLToPath(import.meta.url));
const repo = resolve(here, '..');

/**
 * Built by `npm run e2e:build`, which links the `e2e` Cargo feature and merges
 * `tauri.e2e.conf.json`. A release binary carries neither and cannot be driven.
 */
const appBinary = resolve(
  repo,
  'src-tauri/target/debug',
  process.platform === 'win32' ? 'devnotes.exe' : 'devnotes',
);

/**
 * The WebDriver server runs inside the application, so there is no `tauri-driver`, no
 * msedgedriver and no Chrome DevTools protocol — nor anything those need from their host.
 *
 * ⚠️ The cost shapes every spec file: this provider spawns the application once, from its
 * own `onPrepare`. Every spec file drives the same process, the same SQLite file and the
 * same `preferences.json`, and bumping `maxInstances` does not change it. So the profile
 * is wiped before the runner starts, `before()` buys a fresh front end and nothing more,
 * and a spec file establishes its own preconditions. See `docs/architecture.md`.
 */
const driverProvider = 'embedded';

/** `E2E_LOG_LEVEL=trace` when a session refuses to open; the default keeps runs readable. */
const logLevel = (process.env['E2E_LOG_LEVEL'] ?? 'warn') as NonNullable<WebdriverIO.Config['logLevel']>;

/** The refresh below undoes what a previous file left behind, and there is none yet. */
let aFileHasRun = false;

export const config: WebdriverIO.Config = {
  runner: 'local',
  specs: ['./specs/**/*.e2e.ts'],

  /**
   * ⚠️ The numeric prefix on each spec file is the run order, and it is load-bearing:
   * `01-first-launch` is the only file that meets a virgin profile, and the only one
   * that can resolve the seeded space for the rest.
   */
  maxInstances: 1,

  capabilities: [
    {
      browserName: 'tauri',
      'wdio:enforceWebDriverClassic': true,
      'tauri:options': { application: appBinary },
      'wdio:tauriServiceOptions': { appBinaryPath: appBinary, driverProvider },
    },
  ] as unknown as WebdriverIO.Capabilities[],

  // `embeddedPort` is the base — each worker gets it plus its index.
  services: [['@wdio/tauri-service', { driverProvider, appBinaryPath: appBinary, embeddedPort: 4445 }]],

  framework: 'mocha',
  reporters: ['spec'],
  mochaOpts: { ui: 'bdd', timeout: 120_000 },

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
