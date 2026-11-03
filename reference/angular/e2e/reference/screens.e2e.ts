import { $, $$, browser } from '@wdio/globals';
import { mkdirSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

import type { HttpRequestDraft, NoteDraft } from '@core/ipc/bindings';

import { canvas } from '../pageobjects/canvas.page.js';
import { trash } from '../pageobjects/overlays.page.js';
import { rail } from '../pageobjects/sidebar.page.js';
import { fileMenu, settings, titlebar } from '../pageobjects/titlebar.page.js';
import { emitGlobalAction, hover, press, testid, waitForCanvas } from '../support/app.js';
import { bridge, draft, homeSpaceId } from '../support/bridge.js';

/**
 * The visual reference of the Angular application (egui migration plan, § 5.2): one PNG a
 * screen, under `e2e/reference-captures/`, for the eye to compare the egui screens against.
 * Not a test: a screen it cannot reach is listed in `missed.json`, never a failure. Inactive
 * unless `E2E_CAPTURE=1` (`wdio.conf.ts`), and not one of the scenarios of `e2e/groups.json`.
 */
const here = dirname(fileURLToPath(import.meta.url));
const OUT = join(here, '..', 'reference-captures');

type Theme = 'dark' | 'light';
type Size = readonly [width: number, height: number];

const DEFAULT_SIZE: Size = [1100, 720];
const SIZES: readonly Size[] = [DEFAULT_SIZE, [1920, 1080], [640, 480]];

interface Screen {
  readonly id: string;
  readonly reach: () => Promise<void>;
  /** Puts back a preference `reach` changed, so the next screens do not inherit it. */
  readonly undo?: () => Promise<void>;
}

// The data every screen shows, beside what the first launch seeds.

const TITLES = {
  snippet: 'Relancer le service de paiement',
  note: 'Procédure de mise en production',
  checklist: 'Avant la démonstration',
  calc: 'Budget du déplacement',
  drawing: 'Schéma du réseau',
  trashed: 'Ancienne procédure',
} as const;

const NOTE_BODY = [
  '# Mise en production',
  '',
  'Trois étapes, **dans cet ordre** :',
  '',
  '1. Geler la branche `dev`',
  '2. Lancer la [chaîne de publication](https://exemple.fr/ci)',
  '3. Prévenir l’astreinte',
  '',
  '| Étape | Durée |',
  '| --- | --- |',
  '| Build | 12 min |',
  '| Tests | 25 min |',
  '',
  '```mermaid',
  'flowchart LR',
  '  build --> tests --> publication',
  '```',
  '',
  '> Ne jamais publier un vendredi.',
].join('\n');

async function seed(): Promise<void> {
  const home = await homeSpaceId();
  const work = await bridge.createSpace({ name: 'Projets' });
  const ops = await bridge.createFolder({ spaceId: work.id, name: 'Exploitation' });
  const releases = await bridge.createFolder({ spaceId: work.id, name: 'Livraisons', parentId: ops.id });
  await bridge.recolourFolder(releases.id, 'amber');

  const notes: NoteDraft[] = [
    draft({
      spaceId: home,
      title: TITLES.snippet,
      language: 'sh',
      content: 'kubectl rollout restart deployment/{{service}} -n {{namespace}}',
      source: 'Exploitation / Paiements',
      tags: ['ops', 'kubernetes'],
      pinned: true,
    }),
    draft({ spaceId: home, title: TITLES.note, kind: 'note', content: NOTE_BODY, tags: ['livraison'] }),
    draft({
      spaceId: home,
      title: TITLES.checklist,
      kind: 'checklist',
      items: [
        { text: 'Vider le cache du navigateur', done: true },
        { text: 'Préparer le jeu de données', done: false },
        { text: 'Couper les notifications', done: false },
      ],
      priority: 'high',
    }),
    draft({
      spaceId: home,
      title: TITLES.calc,
      kind: 'calc',
      content: 'train = 2 × 89 €\nhôtel = 3 × 112 €\nrepas = 6 × 18 €\ntotal = train + hôtel + repas',
    }),
    draft({ spaceId: home, title: TITLES.drawing, kind: 'drawing' }),
    draft({
      spaceId: work.id,
      folderId: ops.id,
      title: 'Rotation des clés',
      language: 'sql',
      content: 'SELECT 1;',
    }),
    draft({
      spaceId: work.id,
      folderId: releases.id,
      title: 'Notes de version 2.4',
      kind: 'note',
      content: '## 2.4\n\n- Export CSV',
    }),
    draft({ spaceId: home, title: TITLES.trashed, content: 'obsolète' }),
  ];
  const created = [];
  for (const note of notes) created.push(await bridge.createNote(note));

  const snippet = created[0]!;
  await bridge.updateNote(snippet.id, { content: `${snippet.content}\nkubectl get pods -n {{namespace}}` });
  await bridge.deleteNote(created.at(-1)!.id);

  const api = await bridge.createHttpCollection('API Facturation');
  const auth = await bridge.createHttpFolder(api.id, null, 'Authentification');
  const request = (
    folderId: string | null,
    name: string,
    method: HttpRequestDraft['method'],
  ): HttpRequestDraft => ({
    collectionId: api.id,
    folderId,
    name,
    kind: 'http',
    method,
    document: { url: `https://api.exemple.fr/${name.toLowerCase().replace(/ /g, '-')}`, description: '' },
  });
  await bridge.createHttpRequest(request(auth.id, 'Connexion', 'POST'));
  await bridge.createHttpRequest(request(null, 'Lister les factures', 'GET'));
  await bridge.createHttpEnvironment('Recette');
}

// Gestures, each waiting for what it opens.

async function tap(selector: string): Promise<void> {
  const element = $(selector);
  await element.waitForClickable({ timeout: 10_000 });
  await element.click();
}

async function shows(selector: string): Promise<void> {
  await $(selector).waitForDisplayed({ timeout: 10_000 });
}

const AREA_KEYS = { notes: '1', tools: '2', http: '3' } as const;

/** By its shortcut: the titlebar's switch is drawn only while the rails are hidden. */
async function area(id: keyof typeof AREA_KEYS): Promise<void> {
  await press(AREA_KEYS[id], ['Control']);
  await shows(testid(id === 'notes' ? 'canvas' : `${id}-page`));
}

async function openNote(title: string): Promise<void> {
  await area('notes');
  await canvas.openNote(title);
}

/** An area, then what is done in it. */
function inArea(id: 'notes' | 'tools' | 'http', step: () => Promise<void>): () => Promise<void> {
  return async () => {
    await area(id);
    await step();
  };
}

function inNote(title: string, step: () => Promise<void>): () => Promise<void> {
  return async () => {
    await openNote(title);
    await step();
  };
}

/** The board is a space's: « all spaces » has none. */
async function board(): Promise<void> {
  const work = (await bridge.listSpaces()).find((space) => space.name === 'Projets');
  if (!work) throw new Error('no Projets space');
  await tap(`${testid('space-option')}[data-space-id="${work.id}"]`);
  await tap(testid('view-board'));
  await shows(testid('board'));
}

async function preferences(page: string): Promise<void> {
  await fileMenu.openPreferences();
  await settings.open(page);
}

async function httpRow(name: string): Promise<void> {
  await area('http');
  const rows = await $$(testid('http-node')).getElements();
  for (const row of rows) {
    if ((await row.$(testid('http-node-name')).getText()).endsWith(name)) {
      await row.$(testid('http-node-name')).click();
      return;
    }
  }
  throw new Error(`no HTTP row named ${name}`);
}

async function openRequest(): Promise<void> {
  await httpRow('Lister les factures');
  await shows(testid('http-request-editor'));
}

async function newTab(kind: 'graphql' | 'websocket'): Promise<void> {
  await area('http');
  await tap(testid('http-tab-new-kind'));
  await tap(`${testid('http-new-kind')}[data-kind="${kind}"]`);
  await shows(testid('http-request-editor'));
}

async function tool(id: string): Promise<void> {
  await area('tools');
  await tap(`${testid('tools-entry')}[data-tool="${id}"]`);
  await shows(testid('tool-body'));
  // A worked example where the tool has one: an empty tool shows little of its layout.
  if (await $(testid('tool-sample')).isExisting()) await tap(testid('tool-sample'));
}

const TOOLS = [
  'case',
  'line-breaks',
  'text-stats',
  'diff',
  'hash',
  'password',
  'identifiers',
  'jwt',
  'url-codec',
  'base64',
  'convert',
  'colour',
  'json-generator',
  'identity',
  'lorem',
  'qr',
  'dates',
  'zones',
  'cron',
  'sizes',
  'permissions',
  'checks',
  'cidr',
  'http-status',
  'http-headers',
  'mime-types',
  'ports',
  'signals',
  'ascii',
  'regex-sheet',
  'markdown-sheet',
  'mustache-sheet',
] as const;

/** Drawn in both themes. */
const SCREENS: readonly Screen[] = [
  { id: 'notes-list', reach: () => area('notes') },
  {
    id: 'notes-board',
    reach: inArea('notes', () => board()),
    undo: async () => {
      await tap(testid('view-list'));
      await tap(testid('space-option-all'));
    },
  },
  { id: 'notes-search', reach: inArea('notes', () => canvas.search('production')) },
  { id: 'notes-search-empty', reach: inArea('notes', () => canvas.search('zzzz introuvable')) },
  { id: 'notes-facets', reach: inArea('notes', () => canvas.openFacets()) },
  { id: 'notes-sort-menu', reach: inArea('notes', () => tap(testid('sort-trigger'))) },
  { id: 'notes-new-kind-menu', reach: inArea('notes', () => tap(testid('new-note-kind'))) },
  // The switchers stand in for the rail while it is hidden.
  {
    id: 'notes-space-switcher',
    reach: inArea('notes', async () => {
      await rail.hide();
      await tap(testid('space-switcher'));
    }),
    undo: () => rail.show(),
  },
  {
    id: 'notes-folder-switcher',
    reach: inArea('notes', async () => {
      await rail.hide();
      await tap(testid('folder-switcher'));
    }),
    undo: () => rail.show(),
  },
  {
    id: 'notes-card-menu',
    reach: inArea('notes', async () => {
      await canvas.openCardMenu(TITLES.snippet);
    }),
  },
  {
    id: 'notes-selection',
    reach: async () => {
      await area('notes');
      await canvas.check(TITLES.snippet);
      await canvas.check(TITLES.note);
      await shows(testid('selection-bar'));
    },
  },
  {
    id: 'notes-fields-form',
    reach: async () => {
      await area('notes');
      const card = await canvas.openCardMenu(TITLES.snippet);
      await card.$(testid('note-card-copy')).click();
      await shows(testid('placeholder-form'));
    },
  },
  { id: 'notes-tag-manager', reach: inArea('notes', () => canvas.openTagManager()) },
  { id: 'notes-trash', reach: inArea('notes', () => trash.open()) },
  {
    id: 'notes-palette',
    reach: async () => {
      await emitGlobalAction('palette');
      await shows(testid('palette-input'));
    },
  },
  { id: 'editor-snippet', reach: () => openNote(TITLES.snippet) },
  {
    id: 'editor-snippet-fields',
    reach: inNote(TITLES.snippet, () => tap(testid('placeholder-panel-toggle'))),
  },
  {
    id: 'editor-snippet-revisions',
    reach: inNote(TITLES.snippet, () => tap(testid('revisions-toggle'))),
  },
  { id: 'editor-note', reach: inNote(TITLES.note, () => shows(testid('editor-rich'))) },
  {
    id: 'editor-note-presentation',
    reach: async () => {
      await openNote(TITLES.note);
      await tap(testid('editor-present'));
      await shows(testid('presentation-cover'));
    },
  },
  { id: 'editor-checklist', reach: () => openNote(TITLES.checklist) },
  { id: 'editor-calc', reach: () => openNote(TITLES.calc) },
  { id: 'editor-drawing', reach: inNote(TITLES.drawing, () => shows(testid('drawing-stage'))) },
  { id: 'file-menu', reach: () => tap(testid('file-menu')) },
  { id: 'about-menu', reach: () => tap(testid('about-open')) },
  {
    id: 'libraries',
    reach: async () => {
      await tap(testid('file-menu'));
      await tap(testid('file-libraries'));
    },
  },
  { id: 'settings-general', reach: () => preferences('general') },
  { id: 'settings-shortcuts', reach: () => preferences('shortcuts') },
  { id: 'settings-security', reach: () => preferences('security') },
  { id: 'settings-variables', reach: () => preferences('notes.variables') },
  {
    id: 'settings-change-passphrase',
    reach: async () => {
      await preferences('security');
      await tap(testid('setting-change-passphrase'));
      await shows(testid('change-passphrase'));
    },
  },
  {
    id: 'getting-started',
    reach: async () => {
      await tap(testid('about-open'));
      await tap(testid('about-getting-started'));
      await shows(testid('getting-started'));
    },
  },
  { id: 'tools-home', reach: () => area('tools') },
  {
    id: 'tools-no-match',
    reach: async () => {
      await area('tools');
      await $(testid('tools-search')).setValue('zzzz');
      await shows(testid('tools-no-match'));
    },
  },
  { id: 'http-home', reach: () => area('http') },
  { id: 'http-request', reach: openRequest },
  {
    id: 'http-folder-menu',
    reach: inArea('http', async () => {
      await hover(testid('http-node'));
      await tap(testid('http-node-menu'));
    }),
  },
  { id: 'http-graphql', reach: () => newTab('graphql') },
  { id: 'http-websocket', reach: () => newTab('websocket') },
  { id: 'http-environments', reach: inArea('http', () => tap(testid('http-environments-open'))) },
  { id: 'http-cookies', reach: inArea('http', () => tap(testid('http-cookies-open'))) },
  { id: 'http-history', reach: inArea('http', () => tap(testid('http-history-open'))) },
  {
    id: 'http-code',
    reach: async () => {
      await openRequest();
      await tap(testid('http-code-open'));
    },
  },
];

/** Dark only: the tools share one frame, their bodies are what differs. */
const TOOL_SCREENS: readonly Screen[] = TOOLS.map((id) => ({ id: `tool-${id}`, reach: () => tool(id) }));

/** The screens whose layout answers the window, at the three sizes, dark. */
const SIZED = new Set(['notes-list', 'notes-board', 'editor-note', 'tools-home', 'http-request']);

/** In the compact density, dark, at the default size. */
const COMPACT = new Set([
  'notes-list',
  'notes-board',
  'editor-snippet',
  'editor-checklist',
  'http-request',
  'tools-home',
]);

const missed: { id: string; error: string }[] = [];
const taken: string[] = [];

const firstLine = (error: unknown): string => String(error).split('\n')[0] ?? '';

async function capture(id: string, screen: Screen, theme: Theme, [width, height]: Size): Promise<void> {
  try {
    await browser.refresh();
    await waitForCanvas();
    await browser.setWindowSize(width, height);
    await titlebar.setTheme(theme);
    await screen.reach();
    // Settled on purpose: menus and dialogs fade in, and a capture is not an assertion.
    await browser.pause(700);
    await browser.saveScreenshot(join(OUT, `${id}.png`));
    taken.push(id);
  } catch (error) {
    missed.push({ id, error: firstLine(error) });
  } finally {
    await screen
      .undo?.()
      .catch((error: unknown) => missed.push({ id: `${id} (undo)`, error: firstLine(error) }));
  }
}

async function setDensity(density: 'comfortable' | 'compact'): Promise<void> {
  await browser.refresh();
  await waitForCanvas();
  await fileMenu.openPreferences();
  await settings.setDensity(density);
  await settings.close();
}

describe('visual reference', () => {
  before(async () => {
    mkdirSync(OUT, { recursive: true });
    await waitForCanvas();
    await seed();
  });

  after(() => {
    writeFileSync(join(OUT, 'missed.json'), `${JSON.stringify(missed, null, 2)}\n`);
    writeFileSync(join(OUT, 'taken.json'), `${JSON.stringify(taken, null, 2)}\n`);
  });

  // One test a capture: mocha's timeout is a test's, and a capture never fails one.
  for (const theme of ['dark', 'light'] as const) {
    for (const screen of SCREENS) {
      it(`${screen.id} (${theme})`, () => capture(`${screen.id}--${theme}`, screen, theme, DEFAULT_SIZE));
    }
  }

  for (const screen of TOOL_SCREENS) {
    it(`${screen.id} (dark)`, () => capture(`${screen.id}--dark`, screen, 'dark', DEFAULT_SIZE));
  }

  for (const screen of SCREENS.filter(({ id }) => SIZED.has(id))) {
    for (const size of SIZES.slice(1)) {
      const name = `${screen.id}--dark--${size.join('x')}`;
      it(name, () => capture(name, screen, 'dark', size));
    }
  }

  it('switches to the compact density', async () => {
    try {
      await setDensity('compact');
    } catch (error) {
      missed.push({ id: 'density-compact', error: firstLine(error) });
    }
  });

  for (const screen of SCREENS.filter(({ id }) => COMPACT.has(id))) {
    const name = `${screen.id}--dark--compact`;
    it(name, () => capture(name, screen, 'dark', DEFAULT_SIZE));
  }
});
