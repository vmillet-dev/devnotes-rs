import {
  ChangeDetectionStrategy,
  Component,
  DOCUMENT,
  DestroyRef,
  afterNextRender,
  computed,
  inject,
  signal,
} from '@angular/core';
import { RouterLink } from '@angular/router';

import { clamp, field } from '../core/browser';
import { Lang, downloadPath, homePath } from '../core/lang';
import { Language } from '../core/language';
import { PageProgress, ProgressSegment } from '../core/page-progress';
import { System, VERSION, offers, visitorSystem } from '../core/release';
import { ScrollFrame } from '../core/scroll-frame';
import { Seo } from '../core/seo';
import { REPOSITORY, absolute } from '../core/site';
import { docPath } from '../docs/doc-index';
import { Feature } from './feature';
import { HeroBoard } from './hero-board';
import { MoreSection } from './more-section';
import { ApiScene } from './scenes/api-scene';
import { CodeScene } from './scenes/code-scene';
import { OrganiseScene } from './scenes/organise-scene';
import { SnippetsScene } from './scenes/snippets-scene';
import { ToolsScene } from './scenes/tools-scene';
import { VaultScene } from './scenes/vault-scene';

const FR = {
  title: 'DevNotes : snippets, outils et client API pour développeurs',
  description:
    'Une application de bureau pour développeurs : snippets à champs, notes rangées en tableau, outils hors ligne et client API, chiffrés sur votre machine.',
  headline: 'Le board où vivent vos snippets.',
  lede: "Commandes, requêtes, bouts de code, notes et listes, rangés en espaces et en dossiers. Une trentaine d'outils et un client API à côté, le tout chiffré sur votre machine.",
  downloadFor: 'Télécharger pour',
  docs: 'Lire la documentation',
  facts: ['Gratuit', 'Libre, GPL-3.0', 'Sans compte', 'Hors ligne'],
  /** The fact that links to the source code. */
  sourceFact: 1,
  others: 'Autres systèmes et formats',
  snippets: {
    label: 'Snippets à champs',
    heading: 'Vos commandes, prêtes à coller.',
    steps: [
      'Un snippet avec deux champs',
      'DevNotes demande leurs valeurs',
      'Ctrl Alt P depuis votre terminal',
      'La commande remplie est copiée',
    ],
  },
  organise: {
    label: 'Ranger et retrouver',
    heading: 'Tout est rangé, tout se retrouve.',
    steps: [
      'Un espace et ses dossiers, en tableau',
      'Un dossier s’agrandit à la souris',
      'La recherche cite la ligne trouvée',
      'Les mêmes notes, en liste par date',
    ],
  },
  code: {
    label: 'Le code',
    heading: 'Un vrai éditeur pour vos snippets.',
    steps: ['Choisir le langage au clavier', 'Formater avec Prettier', 'Lire un JSON en graphe'],
  },
  tools: {
    label: 'Outils',
    heading: 'Une trentaine d’outils, sans réseau.',
    steps: [
      'Chercher un outil, Ctrl Maj T',
      'Une expression cron, lue à voix haute',
      'Un JWT décodé, ses dates lues',
      'Le résultat gardé comme une note',
    ],
  },
  api: {
    label: 'Client API',
    heading: 'Testez vos API à côté de vos notes.',
    steps: [
      'Une variable prise dans l’environnement',
      'Envoyer, lire la réponse',
      'Le code pour vingt cibles',
      'Un WebSocket et son journal',
    ],
  },
  vault: {
    label: 'Chiffrement',
    heading: 'Rien ne quitte votre machine.',
    steps: [
      'Sur le disque, la note est chiffrée',
      'Votre phrase de passe la déverrouille',
      'Plusieurs bibliothèques, sauvegardées chaque jour',
    ],
  },
  sections: ['Snippets', 'Ranger', 'Code', 'Outils', 'Client API', 'Chiffrement', 'Et aussi'],
  closing: 'Vos snippets, rangés et à portée de raccourci.',
  closingLede: (version: string) =>
    `DevNotes ${version}, pour Windows et Linux. Gratuit, libre, et rien ne quitte votre machine.`,
  closingDownload: 'Télécharger DevNotes',
};

const EN: typeof FR = {
  title: 'DevNotes: snippets, tools and an API client for developers',
  description:
    'A desktop app for developers: snippets with fields, notes filed on a board, offline tools and an API client, encrypted on your machine.',
  headline: 'The board where your snippets live.',
  lede: 'Commands, queries, bits of code, notes and lists, filed in spaces and folders. Thirty-odd tools and an API client beside them, all encrypted on your machine.',
  downloadFor: 'Download for',
  docs: 'Read the documentation',
  facts: ['Free', 'Open source, GPL-3.0', 'No account', 'Works offline'],
  sourceFact: 1,
  others: 'Other systems and formats',
  snippets: {
    label: 'Snippets with fields',
    heading: 'Your commands, ready to paste.',
    steps: [
      'A snippet with two fields',
      'DevNotes asks for their values',
      'Ctrl Alt P from your terminal',
      'The filled command is copied',
    ],
  },
  organise: {
    label: 'Organise and find',
    heading: 'Everything filed, everything found.',
    steps: [
      'A space and its folders, as a board',
      'A folder grows with the mouse',
      'The search quotes the matching line',
      'The same notes, listed by date',
    ],
  },
  code: {
    label: 'The code',
    heading: 'A real editor for your snippets.',
    steps: ['Pick the language from the keyboard', 'Format with Prettier', 'Read a JSON as a graph'],
  },
  tools: {
    label: 'Tools',
    heading: 'Thirty-odd tools, no network.',
    steps: [
      'Find a tool, Ctrl Shift T',
      'A cron expression, read aloud',
      'A JWT decoded, its dates read',
      'The result kept as a note',
    ],
  },
  api: {
    label: 'API client',
    heading: 'Test your APIs next to your notes.',
    steps: [
      'A variable taken from the environment',
      'Send, read the answer',
      'The code for twenty targets',
      'A WebSocket and its log',
    ],
  },
  vault: {
    label: 'Encryption',
    heading: 'Nothing leaves your machine.',
    steps: [
      'On disk, the note is encrypted',
      'Your passphrase unlocks it',
      'Several libraries, backed up every day',
    ],
  },
  sections: ['Snippets', 'Organise', 'Code', 'Tools', 'API client', 'Encryption', 'And more'],
  closing: 'Your snippets, filed and one shortcut away.',
  closingLede: (version: string) =>
    `DevNotes ${version}, for Windows and Linux. Free, open source, and nothing leaves your machine.`,
  closingDownload: 'Download DevNotes',
};

const TEXT: Readonly<Record<Lang, typeof FR>> = { fr: FR, en: EN };

/** The sections the line under the header follows, in page order. */
const SECTIONS: readonly Omit<ProgressSegment, 'name'>[] = [
  { anchor: 'snippets', tone: 'green' },
  { anchor: 'ranger', tone: 'blue' },
  { anchor: 'code', tone: 'purple' },
  { anchor: 'outils', tone: 'teal' },
  { anchor: 'client-api', tone: 'olive' },
  { anchor: 'chiffrement', tone: 'pink' },
  { anchor: 'et-aussi', tone: 'grey' },
];

@Component({
  selector: 'app-home-page',
  imports: [
    RouterLink,
    HeroBoard,
    Feature,
    SnippetsScene,
    OrganiseScene,
    CodeScene,
    ToolsScene,
    ApiScene,
    VaultScene,
    MoreSection,
  ],
  templateUrl: './home-page.html',
  styleUrl: './home-page.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class HomePage {
  protected readonly field = field;
  protected readonly version = VERSION;
  protected readonly repository = REPOSITORY;
  protected readonly lang = inject(Language).lang;
  protected readonly t = computed(() => TEXT[this.lang()]);
  /** Put forward once the browser says which it is; Windows until then. */
  protected readonly system = signal<System>('windows');
  protected readonly offer = computed(() => {
    const all = offers(this.lang());
    return all.find(({ system }) => system === this.system()) ?? all[0];
  });
  protected readonly links = computed(() => ({
    download: downloadPath(this.lang()),
    docs: docPath('overview', this.lang()),
  }));

  constructor() {
    const progress = inject(PageProgress);
    const document = inject(DOCUMENT);
    const frame = inject(ScrollFrame);
    const destroyRef = inject(DestroyRef);
    const lang = this.lang();
    const t = TEXT[lang];

    inject(Seo).describe({
      title: t.title,
      description: t.description,
      path: homePath(lang),
      lang,
      data: {
        '@type': 'SoftwareApplication',
        name: 'DevNotes',
        applicationCategory: 'DeveloperApplication',
        operatingSystem: 'Windows, Linux',
        softwareVersion: VERSION,
        url: absolute(homePath(lang)),
        downloadUrl: absolute(downloadPath(lang)),
        inLanguage: lang,
        license: 'https://www.gnu.org/licenses/gpl-3.0.html',
        codeRepository: REPOSITORY,
        offers: { '@type': 'Offer', price: '0', priceCurrency: 'EUR' },
      },
    });

    progress.show(SECTIONS.map((section, i) => ({ ...section, name: t.sections[i] ?? '' })));
    destroyRef.onDestroy(() => progress.clear());

    afterNextRender(() => {
      this.system.set(visitorSystem());
      const sections = SECTIONS.map(({ anchor }) => document.getElementById(anchor));
      const stop = frame.watch(() => {
        // Read at the middle of the screen; the gap above a section is not the section yet.
        const probe = innerHeight * 0.5;
        let current = -1;
        const weights: number[] = [];
        const fills: number[] = [];
        sections.forEach((section, i) => {
          if (!section) {
            weights.push(1);
            fills.push(0);
            return;
          }
          const box = section.getBoundingClientRect();
          const start = (section.firstElementChild ?? section).getBoundingClientRect().top;
          weights.push(Math.round(box.height));
          fills.push(Number(clamp((probe - start) / Math.max(1, box.bottom - start)).toFixed(3)));
          if (start <= probe) current = i;
        });
        progress.report({ weights, fills, current });
      });
      destroyRef.onDestroy(stop);
    });
  }
}
