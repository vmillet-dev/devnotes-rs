import { Injectable, inject } from '@angular/core';
import { TranslocoService } from '@jsverse/transloco';
import { firstValueFrom } from 'rxjs';
import { ClockService } from '@core/services/time/clock.service';
import { LibraryPreferencesService } from '@core/services/preferences/library-preferences.service';
import { NotesRepository } from '../data/notes.repository';
import { SpacesRepository } from '../data/spaces.repository';
import { NoteDraft, SampleNote } from '../model/note.model';
import { Space } from '../model/space.model';

/** ⚠️ Exported because `VaultStore` clears it: a library set aside has to seed again. */
export const SEEDED_KEY = 'devnotes.notes.samplesSeeded';

/** The space these drafts belong to does not exist yet; `seed_samples` fills it in. */
const UNFILED = '';

const DEADLINE_DAYS = 7;

/**
 * ⚠️ Untranslatable: Transloco reads `{{name}}` as an interpolation and would replace
 * a snippet's fields with empty strings.
 */
const PSQL_SNIPPET = 'psql -h {{host}} -p {{port=5432}} -U {{user}} -d {{database}}';

const SIGNAL_SNIPPET = `readonly count = signal(0);
readonly doubled = computed(() => this.count() * 2);

increment(): void {
  this.count.update((value) => value + 1);
}`;

/**
 * ⚠️ Indexes into the folders below, not ids: they do not exist until the command that
 * writes them runs. The order is the order the board lays its zones out in.
 */
const SNIPPETS = 0;
const START_HERE = 1;

const KEYS = {
  spaceName: 'notes.samples.space',
  snippetsFolder: 'notes.samples.folders.snippets',
  startHereFolder: 'notes.samples.folders.startHere',
  checklistBoard: 'notes.samples.checklist.board',
  welcomeTitle: 'notes.samples.welcome.title',
  welcomeContent: 'notes.samples.welcome.content',
  welcomeSource: 'notes.samples.welcome.source',
  snippetTitle: 'notes.samples.snippet.title',
  snippetSource: 'notes.samples.snippet.source',
  checklistTitle: 'notes.samples.checklist.title',
  checklistOpen: 'notes.samples.checklist.open',
  checklistCopy: 'notes.samples.checklist.copy',
  checklistFields: 'notes.samples.checklist.fields',
  checklistPalette: 'notes.samples.checklist.palette',
  checklistSpace: 'notes.samples.checklist.space',
  codeTitle: 'notes.samples.code.title',
  codeSource: 'notes.samples.code.source',
  devnotesTag: 'notes.samples.tags.devnotes',
  exampleTag: 'notes.samples.tags.example',
  databaseTag: 'notes.samples.tags.database',
  angularTag: 'notes.samples.tags.angular',
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

    // ⚠️ No space of their own: one command writes the space, its folders and these four
    // notes in a single transaction, and it is the one that decides where they land.
    //
    // ⚠️ One note is deliberately left loose. "No folder" is a legitimate state, and a
    // first launch where everything is filed would teach the opposite.
    const drafts: NoteDraft[] = [
      {
        spaceId: UNFILED,
        folderId: null,
        title: text.welcomeTitle,
        language: 'md',
        content: text.welcomeContent,
        source: text.welcomeSource,
        tags: [text.devnotesTag],
        pinned: true,
        lifecycle: { kind: 'permanent' },
        kind: 'snippet',
        items: [],
      },
      {
        spaceId: UNFILED,
        folderId: null,
        title: text.snippetTitle,
        language: 'sh',
        content: PSQL_SNIPPET,
        source: text.snippetSource,
        tags: [text.databaseTag, text.exampleTag],
        pinned: false,
        lifecycle: { kind: 'permanent' },
        kind: 'snippet',
        items: [],
      },
      {
        spaceId: UNFILED,
        folderId: null,
        title: text.checklistTitle,
        language: 'txt',
        content: '',
        source: '',
        tags: [text.devnotesTag],
        pinned: false,
        lifecycle: { kind: 'permanent' },
        kind: 'checklist',
        items: [
          text.checklistOpen,
          text.checklistCopy,
          text.checklistFields,
          text.checklistPalette,
          text.checklistBoard,
          text.checklistSpace,
        ].map((label) => ({ text: label, done: false })),
      },
      {
        spaceId: UNFILED,
        folderId: null,
        title: text.codeTitle,
        language: 'ts',
        content: SIGNAL_SNIPPET,
        source: text.codeSource,
        tags: [text.angularTag, text.exampleTag],
        pinned: false,
        lifecycle: { kind: 'expires', at: this.deadline() },
        kind: 'snippet',
        items: [],
      },
    ];

    const folders = [text.snippetsFolder, text.startHereFolder];
    // The welcome note and the checklist open the board; the two snippets fill a zone.
    const filing: (number | undefined)[] = [START_HERE, SNIPPETS, undefined, SNIPPETS];
    const notes: SampleNote[] = drafts.map((draft, index) => ({
      folder: filing[index],
      draft,
    }));

    const space = await this.notes.seedSamples(text.spaceName, folders, notes);

    // ⚠️ Written after, and only after: it says "this library has been seeded", which is
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
