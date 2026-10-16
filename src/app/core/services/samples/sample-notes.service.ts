import { Injectable, inject } from '@angular/core';
import { TranslocoService } from '@jsverse/transloco';
import { firstValueFrom } from 'rxjs';
import { ClockService } from '@core/services/time/clock.service';
import { LibraryPreferencesService } from '@core/services/preferences/library-preferences.service';
import { NotesRepository } from '@core/data/notes.repository';
import { SpacesRepository } from '@core/data/spaces.repository';
import { NoteDraft, SampleNote } from '@core/model/note.model';
import { Space } from '@core/model/space.model';

/** Exported because `VaultStore` clears it: a library set aside has to seed again. */
export const SEEDED_KEY = 'devnotes.notes.samplesSeeded';

/** The space these drafts belong to does not exist yet; `seed_samples` fills it in. */
const UNFILED = '';

const DEADLINE_DAYS = 7;

/**
 * ⚠️ Untranslatable: Transloco reads `{{name}}` as an interpolation and would replace
 * a snippet's fields with empty strings.
 */
const PSQL_SNIPPET = 'psql -h {{host}} -p {{port=5432}} -U {{user}} -d {{database}}';

/** A document to open as a graph: data reads the same in every language. */
const API_RESPONSE = `{
  "service": "billing",
  "version": "1.4.2",
  "healthy": true,
  "replicas": 3,
  "endpoints": [
    { "path": "/invoices", "method": "GET", "p95Ms": 42 },
    { "path": "/invoices", "method": "POST", "p95Ms": 118 },
    { "path": "/health", "method": "GET", "p95Ms": 3 }
  ],
  "owner": { "team": "payments", "onCall": "@oncall" }
}`;

/** Written on one line on purpose: Shift+Alt+F is what lays it out. */
const UNFORMATTED =
  'const retries={max:3,delayMs:250};export async function withRetry<T>(task:()=>Promise<T>,left=retries.max):Promise<T>{try{return await task()}catch(error){if(left<=0)throw error;await new Promise(r=>setTimeout(r,retries.delayMs));return withRetry(task,left-1)}}';

/**
 * Indexes into the folders below, not ids: they do not exist until the command that
 * writes them runs. The order is the order the board lays its zones out in.
 */
const SNIPPETS = 0;
const START_HERE = 1;

const KEYS = {
  spaceName: 'notes.samples.space',
  snippetsFolder: 'notes.samples.folders.snippets',
  startHereFolder: 'notes.samples.folders.startHere',
  welcomeTitle: 'notes.samples.welcome.title',
  welcomeContent: 'notes.samples.welcome.content',
  welcomeSource: 'notes.samples.welcome.source',
  snippetTitle: 'notes.samples.snippet.title',
  snippetSource: 'notes.samples.snippet.source',
  jsonTitle: 'notes.samples.json.title',
  jsonSource: 'notes.samples.json.source',
  releaseTitle: 'notes.samples.release.title',
  releaseContent: 'notes.samples.release.content',
  checklistTitle: 'notes.samples.checklist.title',
  checklistOpen: 'notes.samples.checklist.open',
  checklistFields: 'notes.samples.checklist.fields',
  checklistGraph: 'notes.samples.checklist.graph',
  checklistSearch: 'notes.samples.checklist.search',
  checklistFormat: 'notes.samples.checklist.format',
  checklistTools: 'notes.samples.checklist.tools',
  checklistPalette: 'notes.samples.checklist.palette',
  checklistBoard: 'notes.samples.checklist.board',
  checklistKind: 'notes.samples.checklist.kind',
  formatTitle: 'notes.samples.format.title',
  formatSource: 'notes.samples.format.source',
  devnotesTag: 'notes.samples.tags.devnotes',
  exampleTag: 'notes.samples.tags.example',
  databaseTag: 'notes.samples.tags.database',
  jsonTag: 'notes.samples.tags.json',
  typescriptTag: 'notes.samples.tags.typescript',
} as const;

type SampleTexts = Record<keyof typeof KEYS, string>;

/**
 * A virgin database has no space, so not even a note can be created. Their text comes
 * from the front end, so they arrive in the language the application starts in.
 */
@Injectable({ providedIn: 'root' })
export class SampleNotesService {
  private readonly notes = inject(NotesRepository);
  private readonly spaces = inject(SpacesRepository);
  private readonly preferences = inject(LibraryPreferencesService);
  private readonly transloco = inject(TranslocoService);
  private readonly clock = inject(ClockService);

  /**
   * ⚠️ Two guards, not one: the marker alone would re-seed anyone whose preferences file
   * went missing, "no space at all" alone the day the last space disappears. Together
   * they only ever match a database that has never been written to.
   */
  async seedIfFirstRun(): Promise<Space | null> {
    if (this.preferences.read(SEEDED_KEY) !== null) return null;

    try {
      if ((await this.spaces.loadAll()).length > 0) {
        this.preferences.write(SEEDED_KEY, 'skipped');
        return null;
      }

      return await this.seed();
    } catch {
      // No bridge (jsdom), or a database that will not open: the canvas reports it.
      return null;
    }
  }

  private async texts(): Promise<SampleTexts> {
    const names = Object.keys(KEYS) as (keyof typeof KEYS)[];
    const translated = await firstValueFrom(this.transloco.selectTranslate<string[]>(Object.values(KEYS)));

    return Object.fromEntries(names.map((name, index) => [name, translated[index] ?? ''])) as SampleTexts;
  }

  private async seed(): Promise<Space> {
    const text = await this.texts();
    const sample = (draft: Partial<NoteDraft> & Pick<NoteDraft, 'title' | 'kind'>): NoteDraft => ({
      spaceId: UNFILED,
      folderId: null,
      language: 'txt',
      content: '',
      source: '',
      tags: [],
      pinned: false,
      lifecycle: { kind: 'permanent' },
      priority: 'none',
      items: [],
      ...draft,
    });

    // No space of their own: one command writes the space, its folders and these notes in
    // a single transaction, and it is the one that decides where they land. Each shows one
    // thing, short enough to read without scrolling.
    const drafts: NoteDraft[] = [
      sample({
        title: text.welcomeTitle,
        kind: 'note',
        content: text.welcomeContent,
        source: text.welcomeSource,
        tags: [text.devnotesTag],
        pinned: true,
      }),
      sample({
        title: text.snippetTitle,
        kind: 'snippet',
        language: 'sh',
        content: PSQL_SNIPPET,
        source: text.snippetSource,
        tags: [text.databaseTag, text.exampleTag],
      }),
      sample({
        title: text.jsonTitle,
        kind: 'snippet',
        language: 'json',
        content: API_RESPONSE,
        source: text.jsonSource,
        tags: [text.jsonTag, text.exampleTag],
      }),
      sample({
        title: text.releaseTitle,
        kind: 'note',
        content: text.releaseContent,
        tags: [text.devnotesTag],
      }),
      sample({
        title: text.checklistTitle,
        kind: 'checklist',
        tags: [text.devnotesTag],
        lifecycle: { kind: 'expires', at: this.deadline() },
        priority: 'high',
        items: [
          text.checklistOpen,
          text.checklistFields,
          text.checklistGraph,
          text.checklistSearch,
          text.checklistFormat,
          text.checklistTools,
          text.checklistPalette,
          text.checklistBoard,
          text.checklistKind,
        ].map((label) => ({ text: label, done: false })),
      }),
      sample({
        title: text.formatTitle,
        kind: 'snippet',
        language: 'ts',
        content: UNFORMATTED,
        source: text.formatSource,
        tags: [text.typescriptTag, text.exampleTag],
      }),
    ];

    const folders = [text.snippetsFolder, text.startHereFolder];
    // The two Notes open the board; the snippets fill a zone. The todo list is left loose on
    // purpose: "no folder" is a legitimate state, and the first launch shows it.
    const filing: (number | undefined)[] = [START_HERE, SNIPPETS, SNIPPETS, START_HERE, undefined, SNIPPETS];
    const notes: SampleNote[] = drafts.map((draft, index) => ({
      folder: filing[index],
      draft,
    }));

    const space = await this.notes.seedSamples(text.spaceName, folders, notes);

    // Written after, and only after: it says "this library has been seeded", which is
    // now something observed rather than hoped for. An interrupted seeding rolls back
    // whole, so the next launch finds no marker and no space, and seeds again.
    this.preferences.write(SEEDED_KEY, 'true');

    return space;
  }

  /** End of the local day: midnight would make a note dated today expired on the spot. */
  private deadline(): Date {
    const at = new Date(this.clock.now());
    at.setDate(at.getDate() + DEADLINE_DAYS);
    at.setHours(23, 59, 59, 999);

    return at;
  }
}
