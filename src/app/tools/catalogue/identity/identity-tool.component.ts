import { ChangeDetectionStrategy, Component, computed, inject, signal } from '@angular/core';
import { toSignal } from '@angular/core/rxjs-interop';
import { TranslocoPipe, TranslocoService } from '@jsverse/transloco';
import { ToolsRepository } from '@core/data/tools.repository';
import { Country, Identifier, PeopleFormat, Sex } from '@core/model/tool-answers.model';
import { ClipboardService } from '@core/services/clipboard/clipboard.service';
import { FileDialogService } from '@core/services/dialogs/file-dialog.service';
import { ErrorNotifier } from '@core/services/errors/error-notifier.service';
import { StatusNotifier } from '@core/services/notifications/status.service';
import { ClockService } from '@core/services/time/clock.service';
import { liveResult } from '@core/services/tools/live-result';
import { Tool, ToolAction, ToolResult } from '@core/services/tools/tool.model';
import { toolState } from '@core/services/tools/tool-sessions';
import { ChoiceMenuComponent, ChoiceOption } from '@shared/controls/choice-menu/choice-menu.component';
import {
  Segment,
  SegmentedChoiceComponent,
} from '@shared/controls/segmented-choice/segmented-choice.component';
import { IconComponent } from '@shared/icon/icon.component';
import { CopyValueComponent } from '@tools/ui/copy-value/copy-value.component';
import { ResultRowComponent } from '@tools/ui/result-row/result-row.component';

const ANY = 'any';

const SEXES: readonly Segment[] = ([ANY, 'F', 'M'] as const).map((id) => ({
  id,
  labelKey: `tools.identity.sexes.${id}`,
}));

const FORMATS: readonly Segment[] = (['json', 'csv'] as const).map((id) => ({
  id,
  labelKey: `tools.identity.formats.${id}`,
}));

const COUNTRY_CODES: readonly Country[] = [
  'FR',
  'BE',
  'DE',
  'ES',
  'IT',
  'PT',
  'NL',
  'CH',
  'GB',
  'PL',
  'SE',
  'US',
  'CA',
];
const COUNTRIES: readonly ChoiceOption[] = COUNTRY_CODES.map((code) => ({
  id: code,
  name: `tools.identity.countries.${code}`,
  nameIsKey: true,
}));

/** What the options are back to on Vider, and what a first visit opens on. */
const DEFAULTS = { ageMin: 18, ageMax: 80, people: 10 } as const;
export const AGE_BOUNDS = { min: 0, max: 110 } as const;
export const PEOPLE_BOUNDS = { min: 1, max: 100 } as const;

/** The seed a sample draws from: the same person for whoever opens the tool. */
const SAMPLE_SEED = 'devnotes';

/**
 * What a developer does differently with the value: a phone that may ring at someone's, a
 * number that is a format with no key to check it by. Said on its row, not in a note under all.
 */
type Mark = 'mayRing' | 'unkeyed';

interface Row {
  readonly id: string;
  /** A translation key, or the identifier's own name: a « SIRET » is called so everywhere. */
  readonly labelKey: string;
  readonly value: string;
  readonly mark: Mark | null;
}

/** The whole years between a birth date and now, read as calendar dates. */
export function ageOn(birth: string, now: Date): number {
  const [year = 0, month = 1, day = 1] = birth.split('-').map(Number);
  const before = now.getMonth() + 1 < month || (now.getMonth() + 1 === month && now.getDate() < day);
  return now.getFullYear() - year - (before ? 1 : 0);
}

/**
 * Rust draws the person and computes every number; this draws the card. ⚠️ It **evokes** an
 * identity card and imitates none: a monogram where a photo would be, no emblem, no
 * machine-readable zone.
 * Nothing here is a secret: the options are the session's, the person is an output.
 */
@Component({
  selector: 'app-identity-tool',
  imports: [
    ChoiceMenuComponent,
    CopyValueComponent,
    IconComponent,
    ResultRowComponent,
    SegmentedChoiceComponent,
    TranslocoPipe,
  ],
  templateUrl: './identity-tool.component.html',
  styleUrl: './identity-tool.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class IdentityToolComponent implements Tool {
  private readonly repository = inject(ToolsRepository);
  private readonly clipboard = inject(ClipboardService);
  private readonly files = inject(FileDialogService);
  private readonly notifier = inject(ErrorNotifier);
  private readonly status = inject(StatusNotifier);
  private readonly clock = inject(ClockService);
  private readonly transloco = inject(TranslocoService);

  protected readonly country = toolState<Country>('identity.country', 'FR');
  /** `null` draws it. */
  protected readonly sex = toolState<Sex | null>('identity.sex', null);
  protected readonly ageMin = toolState<number>('identity.ageMin', DEFAULTS.ageMin);
  protected readonly ageMax = toolState<number>('identity.ageMax', DEFAULTS.ageMax);
  /** `null` draws one: the answer says which, and the pin writes it here. */
  protected readonly seed = toolState<string | null>('identity.seed', null);
  protected readonly people = toolState<number>('identity.people', DEFAULTS.people);
  protected readonly format = toolState<PeopleFormat>('identity.format', 'json');
  /** The countries a batch adds to the card's own, which is always in it. */
  protected readonly batchCountries = toolState<readonly Country[]>('identity.batchCountries', []);

  /** A new draw with the same options: part of the request, so the request changes. */
  private readonly draw = signal(0);

  protected readonly sexes = SEXES;
  protected readonly formats = FORMATS;
  protected readonly countries = COUNTRIES;
  protected readonly countryCodes = COUNTRY_CODES;
  protected readonly ages = AGE_BOUNDS;
  protected readonly peopleBounds = PEOPLE_BOUNDS;
  protected readonly anySex = ANY;

  /** The dates are written in the language on screen: its file loaded, they are written again. */
  private readonly lang = toSignal(this.transloco.langChanges$, {
    initialValue: this.transloco.getActiveLang(),
  });

  private readonly request = computed(() => ({
    country: this.country(),
    sex: this.sex(),
    ageMin: this.ageMin(),
    ageMax: this.ageMax(),
    seed: this.seed(),
  }));

  private readonly outcome = liveResult(
    () => ({ ...this.request(), draw: this.draw() }),
    ({ draw: _draw, ...request }) => this.repository.generateIdentity(request),
  );

  protected readonly answer = computed(() => {
    const outcome = this.outcome.value();
    return outcome?.kind === 'drawn' ? outcome.answer : null;
  });

  /** ⚠️ The lists are files beside the application: one that cannot be read is said, in place of a card. */
  protected readonly unreadable = computed(() => {
    const outcome = this.outcome.value();
    return outcome?.kind === 'unreadable' ? outcome.list : null;
  });

  protected readonly person = computed(() => this.answer()?.person ?? null);

  /** Where a photo would be: the first letters of the first given name and of the surname. */
  protected readonly monogram = computed(() => {
    const person = this.person();
    if (!person) return '';
    return [person.givenNames[0] ?? '', person.surname]
      .map((name) => [...name][0] ?? '')
      .join('')
      .toUpperCase();
  });

  protected readonly born = computed(() => {
    const person = this.person();
    if (!person) return null;
    const [year = 0, month = 1, day = 1] = person.birth.date.split('-').map(Number);
    return {
      date: new Intl.DateTimeFormat(this.lang(), { dateStyle: 'long', timeZone: 'UTC' }).format(
        new Date(Date.UTC(year, month - 1, day)),
      ),
      age: ageOn(person.birth.date, this.clock.now()),
    };
  });

  protected readonly contact = computed<readonly Row[]>(() => {
    const person = this.person();
    if (!person) return [];
    const phones = this.answer()?.fictionPhones ? null : 'mayRing';
    return [
      {
        id: 'address',
        labelKey: 'tools.identity.fields.address',
        value: person.address.formatted,
        mark: null,
      },
      { id: 'phone', labelKey: 'tools.identity.fields.phone', value: person.phone, mark: phones },
      { id: 'mobile', labelKey: 'tools.identity.fields.mobile', value: person.mobile, mark: phones },
      { id: 'email', labelKey: 'tools.identity.fields.email', value: person.email, mark: null },
    ];
  });

  protected readonly bank = computed<readonly Row[]>(() => {
    const bank = this.person()?.bank;
    if (!bank) return [];
    // A country without an IBAN shows what it writes instead, and no empty row.
    return [
      { id: 'bank', labelKey: 'tools.identity.fields.bank', value: bank.name, mark: null },
      ...(bank.iban === null
        ? []
        : [{ id: 'iban', labelKey: 'tools.identity.fields.iban', value: bank.iban, mark: null }]),
      ...(bank.bic === null
        ? []
        : [{ id: 'bic', labelKey: 'tools.identity.fields.bic', value: bank.bic, mark: null }]),
      ...this.named(bank.local),
    ];
  });

  protected readonly company = computed<readonly Row[]>(() => {
    const company = this.person()?.company;
    if (!company) return [];
    return [
      { id: 'company', labelKey: 'tools.identity.fields.company', value: company.name, mark: null },
      ...this.named(company.identifiers),
    ];
  });

  /** A country whose firms have no number with a key says so, rather than show one made up. */
  protected readonly firmHasNoNumber = computed(() => this.person()?.company.identifiers.length === 0);

  /** The numbers that are the person's own, on the card itself. */
  protected readonly personal = computed<readonly Row[]>(() => this.named(this.person()?.identifiers ?? []));

  readonly actions = computed<readonly ToolAction[]>(() => [
    { id: 'draw', labelKey: 'tools.identity.again', disabled: false, run: () => this.again() },
  ]);

  readonly result = computed<ToolResult | null>(() => {
    const answer = this.answer();
    if (!answer) return null;
    const { givenNames, surname } = answer.person;
    return {
      title: { key: 'tools.identity.noteTitle', params: { name: `${givenNames[0] ?? ''} ${surname}` } },
      kind: 'snippet',
      language: 'json',
      content: answer.json,
    };
  });

  sample(): void {
    this.country.set('FR');
    this.sex.set(null);
    this.ageMin.set(DEFAULTS.ageMin);
    this.ageMax.set(DEFAULTS.ageMax);
    this.seed.set(SAMPLE_SEED);
  }

  /** Back to the options a first visit has, and a fresh draw. */
  clear(): void {
    this.country.set('FR');
    this.batchCountries.set([]);
    this.sex.set(null);
    this.ageMin.set(DEFAULTS.ageMin);
    this.ageMax.set(DEFAULTS.ageMax);
    this.seed.set(null);
    this.people.set(DEFAULTS.people);
    this.again();
  }

  protected again(): void {
    this.draw.update((draw) => draw + 1);
  }

  protected onCountry(id: string | null): void {
    const country = COUNTRY_CODES.find((code) => code === id);
    if (country) this.country.set(country);
  }

  protected inBatch(code: Country): boolean {
    return code === this.country() || this.batchCountries().includes(code);
  }

  protected toggleBatch(code: Country): void {
    this.batchCountries.update((countries) =>
      countries.includes(code) ? countries.filter((each) => each !== code) : [...countries, code],
    );
  }

  protected onSex(id: string): void {
    this.sex.set(id === ANY ? null : (id as Sex));
  }

  protected onAge(bound: 'min' | 'max', event: Event): void {
    const typed = Number.parseInt((event.target as HTMLInputElement).value, 10);
    if (!Number.isFinite(typed)) return;
    (bound === 'min' ? this.ageMin : this.ageMax).set(
      Math.max(AGE_BOUNDS.min, Math.min(typed, AGE_BOUNDS.max)),
    );
  }

  protected onSeed(event: Event): void {
    const typed = (event.target as HTMLInputElement).value.trim();
    this.seed.set(typed === '' ? null : typed);
  }

  /** The seed just used, written down — or, pinned already, let go. */
  protected togglePin(used: string | null): void {
    this.seed.set(this.seed() === null ? used : null);
  }

  protected onPeople(event: Event): void {
    const typed = Number.parseInt((event.target as HTMLInputElement).value, 10);
    if (Number.isFinite(typed)) {
      this.people.set(Math.max(PEOPLE_BOUNDS.min, Math.min(typed, PEOPLE_BOUNDS.max)));
    }
  }

  protected onFormat(id: string): void {
    this.format.set(id as PeopleFormat);
  }

  protected async copyJson(): Promise<void> {
    const json = this.answer()?.json;
    if (json && (await this.clipboard.copy(json))) {
      this.status.notify({ key: 'tools.identity.jsonCopied' });
    }
  }

  /** A batch is a file, written by Rust where the user says: a hundred people are no note. */
  protected async export(): Promise<void> {
    const used = this.answer()?.seed ?? null;
    const path = await this.files.chooseDestination(`identities.${this.format()}`);
    if (path === null) return;

    // The seed of the card on screen: the file opens on that very person.
    const written = await this.notifier.attempt('errors.toolFailed', () =>
      this.repository.exportIdentities(
        { ...this.request(), seed: this.seed() ?? used },
        { countries: [...this.batchCountries()], count: this.people(), format: this.format() },
        path,
      ),
    );
    if (written?.kind === 'written') {
      this.status.notify({ key: 'tools.identity.exported', params: { count: written.people } });
    } else if (written?.kind === 'failed') {
      this.notifier.notify({ ref: { key: `tools.files.${written.problem}` } });
    } else if (written?.kind === 'unreadable') {
      this.notifier.notify({
        ref: { key: `tools.identity.lists.${written.list.problem}`, params: { file: written.list.file } },
      });
    }
  }

  /** Rust names the numbers of this card that carry no key; this only marks their rows. */
  private named(identifiers: readonly Identifier[]): Row[] {
    const unkeyed = this.answer()?.unkeyed ?? [];
    return identifiers.map((identifier) => ({
      id: identifier.kind,
      labelKey: `tools.identity.identifiers.${identifier.kind}`,
      value: identifier.value,
      mark: unkeyed.includes(identifier.kind) ? 'unkeyed' : null,
    }));
  }
}
