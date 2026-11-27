import { ChangeDetectionStrategy, Component, afterNextRender, computed, inject, signal } from '@angular/core';
import { RouterLink } from '@angular/router';

import { Lang, downloadPath, french, homePath } from '../core/lang';
import { Language } from '../core/language';
import { Offer, RELEASE_PAGE, System, VERSION, offers, systems, visitorSystem } from '../core/release';
import { Seo } from '../core/seo';
import { REPOSITORY, absolute } from '../core/site';
import { ThemeStore } from '../core/theme.store';
import { DaySection } from './day/day-section';
import { Run, Runs } from './runs';

interface HomeText {
  readonly title: string;
  readonly description: string;
  readonly section: string;
  readonly headline: string;
  readonly lede: string;
  readonly downloadFor: string;
  readonly others: string;
  readonly free: string;
  readonly facts: readonly string[];
  readonly alt: string;
  readonly legendName: string;
  /** What each mark on the capture points at, in its order. */
  readonly legend: readonly { readonly name: string; readonly text: readonly Run[] }[];
  readonly get: string;
  readonly getHeading: (version: string) => string;
  readonly getLede: string;
  readonly codeHeading: string;
  readonly codeText: string;
  readonly source: string;
  readonly notes: string;
}

const FR: HomeText = {
  title: 'DevNotes : snippets, notes et client API pour développeurs',
  description:
    'Application de bureau pour développeurs : snippets à champs, notes Markdown, listes, calculs, dessins, client API et 32 outils, chiffrés sur votre machine.',
  section: "L'application",
  headline: 'Vos snippets, vos notes, vos requêtes.',
  lede: "Une application de bureau pour tout ce qu'un développeur garde sous la main : des snippets à champs, des notes en Markdown, des listes, des calculs et des dessins, avec un client API et 32 outils à côté. Chiffrée sur votre machine, sans compte.",
  downloadFor: 'Télécharger pour',
  others: 'Autres systèmes et formats',
  free: 'Gratuit, sous GPL-3.0',
  facts: ['Sans compte', 'Hors ligne'],
  alt: 'DevNotes : la liste des notes, avec ses espaces à gauche, la recherche en haut et des cartes de snippet, de requête SQL, de schéma, de Calcul et de liste à cocher.',
  legendName: 'Ce que montre la capture',
  legend: [
    { name: 'Trois zones', text: ['Les notes, les 32 outils et le client API, dans la même fenêtre.'] },
    {
      name: 'Espaces et dossiers',
      text: ['Vos notes rangées par projet, avec des tags, en liste ou sur un tableau.'],
    },
    {
      name: 'Une recherche',
      text: [
        { keys: ['Ctrl', 'K'] },
        ' lit les titres, le code, les listes et les tags, sans tenir compte des accents.',
      ],
    },
    {
      name: 'Des snippets à champs',
      text: [
        { code: '{{namespace}}' },
        ' se remplit au moment de copier, même depuis une autre application avec ',
        { keys: ['Ctrl', 'Alt', 'P'] },
        '.',
      ],
    },
    {
      name: 'Des Calculs',
      text: ["Un résultat par ligne, en euros, en heures ou en octets, refait dès qu'un chiffre change."],
    },
    { name: 'Des listes', text: ['Leur carte montre où vous en êtes, sans ouvrir la liste.'] },
  ],
  get: 'Télécharger',
  getHeading: (version) => `DevNotes ${version}, pour ${systems('fr')}`,
  getLede:
    "Une application, sans compte ni serveur. Votre bibliothèque reste sur votre machine, chiffrée, et l'application se met à jour d'elle-même.",
  codeHeading: 'Le code',
  codeText:
    'DevNotes est écrit en Rust et publié sous GPL-3.0 : tout le code est lisible, et chaque version a ses notes.',
  source: 'Code source',
  notes: 'Notes de version',
};

const EN: HomeText = {
  title: 'DevNotes: snippets, notes and an API client for developers',
  description:
    'A desktop app for developers: snippets with fields, Markdown notes, lists, calculations, drawings, an API client and 32 tools, encrypted on your machine.',
  section: 'The application',
  headline: 'Your snippets, your notes, your requests.',
  lede: 'A desktop application for everything a developer keeps at hand: snippets with fields, Markdown notes, lists, calculations and drawings, with an API client and 32 tools beside them. Encrypted on your machine, no account.',
  downloadFor: 'Download for',
  others: 'Other systems and formats',
  free: 'Free, under the GPL-3.0',
  facts: ['No account', 'Works offline'],
  alt: 'DevNotes: the list of notes, with its spaces on the left, the search at the top, and cards of a snippet, an SQL query, a diagram, a calculation and a to-do list.',
  legendName: 'What the capture shows',
  legend: [
    { name: 'Three areas', text: ['The notes, the 32 tools and the API client, in the same window.'] },
    {
      name: 'Spaces and folders',
      text: ['Your notes filed by project, with tags, as a list or on a board.'],
    },
    {
      name: 'A search',
      text: [{ keys: ['Ctrl', 'K'] }, ' reads titles, code, lists and tags, ignoring case and accents.'],
    },
    {
      name: 'Snippets with fields',
      text: [
        { code: '{{namespace}}' },
        ' is filled in when you copy, even from another application with ',
        { keys: ['Ctrl', 'Alt', 'P'] },
        '.',
      ],
    },
    {
      name: 'Calculations',
      text: ['One result a line, in euros, hours or bytes, redone as soon as a figure changes.'],
    },
    { name: 'Lists', text: ['Their card shows where you stand, without opening the list.'] },
  ],
  get: 'Download',
  getHeading: (version) => `DevNotes ${version}, for ${systems('en')}`,
  getLede:
    'One application, no account and no server. Your library stays on your machine, encrypted, and the application updates itself.',
  codeHeading: 'The code',
  codeText:
    'DevNotes is written in Rust and released under the GPL-3.0: all of its code can be read, and every version has its notes.',
  source: 'Source code',
  notes: 'Release notes',
};

const TEXT: Readonly<Record<Lang, HomeText>> = { fr: french(FR), en: EN };

/**
 * Where each legend's mark stands on the capture, in percent of its width and height: on the
 * corner of what it names, never over its words. The two languages lay the toolbar out
 * differently: English keeps the search on the first row.
 */
const MARKS: Readonly<Record<Lang, readonly (readonly [number, number])[]>> = {
  fr: [
    [16, 7.8],
    [16, 23.1],
    [19.3, 13.2],
    [19.7, 26.9],
    [19.7, 77.8],
    [45.7, 77.8],
  ],
  en: [
    [16, 7.8],
    [16, 23.1],
    [37.9, 7.3],
    [19.7, 21],
    [19.7, 72],
    [45.7, 72],
  ],
};

/**
 * The capture is `app::screens`' notes list, drawn at twice its size, in each language and theme.
 * A phone shows its top left corner at more than twice the width of the screen (home-page.scss).
 */
const SHOT = { width: 2168, height: 1362, widths: [1084, 2168] } as const;
const SIZES = '(min-width: 1380px) 1300px, (max-width: 640px) 242vw, 100vw';

@Component({
  selector: 'app-home-page',
  imports: [RouterLink, DaySection, Runs],
  templateUrl: './home-page.html',
  styleUrl: './home-page.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class HomePage {
  protected readonly version = VERSION;
  protected readonly repository = REPOSITORY;
  protected readonly releasePage = RELEASE_PAGE;
  protected readonly shot = SHOT;
  protected readonly sizes = SIZES;
  protected readonly lang = inject(Language).lang;
  protected readonly t = computed(() => TEXT[this.lang()]);
  protected readonly marks = computed(() => MARKS[this.lang()]);
  /** The legend's item under the pointer, whose mark on the capture grows. */
  protected readonly pointed = signal(-1);
  /** Put forward once the browser says which it is; Windows until then. */
  protected readonly system = signal<System>('windows');
  protected readonly offers = computed(() =>
    offers(this.lang()).map((offer) => ({ ...offer, files: [offer.primary, ...offer.others] })),
  );
  protected readonly offer = computed(
    () => this.offers().find(({ system }) => system === this.system()) ?? this.offers()[0],
  );
  protected readonly systems = computed(() => systems(this.lang()));
  protected readonly downloadPath = computed(() => downloadPath(this.lang()));

  /**
   * When the capture's light file is the one shown: from the system's theme before any script,
   * then from the site's own once it is known. The browser fetches one file, the one on screen.
   */
  private readonly connected = signal(false);
  private readonly theme = inject(ThemeStore);
  protected readonly lightWhen = computed(() => {
    if (!this.connected()) return '(prefers-color-scheme: light)';
    return this.theme.theme() === 'light' ? 'all' : 'not all';
  });

  constructor() {
    const lang = this.lang();
    const t = TEXT[lang];

    inject(Seo).describe({
      title: t.title,
      description: t.description,
      path: homePath(lang),
      lang,
      data: [
        {
          '@type': 'SoftwareApplication',
          name: 'DevNotes',
          applicationCategory: 'DeveloperApplication',
          operatingSystem: offers(lang)
            .map(({ label }) => label)
            .join(', '),
          softwareVersion: VERSION,
          description: t.description,
          screenshot: absolute(this.file('dark', SHOT.width)),
          url: absolute(homePath(lang)),
          downloadUrl: absolute(downloadPath(lang)),
          inLanguage: lang,
          license: 'https://www.gnu.org/licenses/gpl-3.0.html',
          codeRepository: REPOSITORY,
          offers: { '@type': 'Offer', price: '0', priceCurrency: 'EUR' },
        },
        { '@type': 'WebSite', name: 'DevNotes', url: absolute(homePath(lang)), inLanguage: lang },
      ],
    });

    afterNextRender(() => {
      this.system.set(visitorSystem());
      this.connected.set(true);
    });
  }

  protected file(theme: 'dark' | 'light', width: number): string {
    return `/shots/notes-${this.lang()}-${theme}-${width}.webp`;
  }

  protected srcset(theme: 'dark' | 'light'): string {
    return SHOT.widths.map((width) => `${this.file(theme, width)} ${width}w`).join(', ');
  }

  /**
   * What the button's file is: its size, and for a Mac its chip, since the browser does not say
   * which a Mac has. An Intel Mac finds its own file on the download page.
   */
  protected note(offer: Offer): string {
    const parts = offer.system === 'macos' ? [offer.primary.name, offer.primary.size] : [offer.primary.size];
    return parts.filter(Boolean).join(', ');
  }
}
