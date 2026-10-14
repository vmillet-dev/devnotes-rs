import {
  ChangeDetectionStrategy,
  Component,
  DestroyRef,
  ElementRef,
  afterNextRender,
  computed,
  inject,
  signal,
  viewChild,
} from '@angular/core';

import { canObserve, prefersReducedMotion, sleep } from '../core/browser';
import { Lang } from '../core/lang';
import { Language } from '../core/language';

const FR = {
  label: 'Et aussi',
  heading: 'Et tout ce qui va autour.',
  notes: {
    title: 'Des notes',
    line: 'Écrites mises en forme, enregistrées en Markdown.',
    heading: 'Rotation du certificat',
    items: ["Émettre depuis l'ACME", 'Recharger nginx'],
  },
  lists: {
    title: 'Des listes',
    line: 'À cocher, avec leur avancement sur la carte.',
    items: ['Geler la branche', 'Lancer la CI', 'Publier la release', "Prévenir l'équipe"],
  },
  trash: {
    title: 'Une corbeille',
    before: "Trente jours pour changer d'avis, et",
    after: 'juste après une suppression.',
    toast: 'Note mise à la corbeille',
    undo: 'Annuler',
  },
  versions: {
    title: 'Des versions',
    line: "Les vingt dernières versions d'un snippet, à relire et à rétablir.",
    items: [
      ['Version actuelle', "à l'instant"],
      ['Avant le formatage', 'il y a 2 min'],
      ['Port changé', 'hier'],
    ] as readonly (readonly [string, string])[],
  },
  attachments: {
    title: 'Des pièces jointes',
    line: 'Des fichiers et des images collées, chiffrés avec la note.',
    items: ['schema-factures.png', 'contrat-api.pdf'],
  },
  selection: {
    title: 'La sélection multiple',
    line: "Déplacez, étiquetez ou supprimez vingt notes d'un coup.",
    count: '20 sélectionnées',
    actions: ['Déplacer vers', 'Priorité', '+ tag'],
  },
  transfer: {
    title: 'Import et export',
    before: 'Une archive',
    after: ', protégée si vous le voulez, ou la sélection copiée en Markdown.',
    items: ['travail-2026-10-14.devnotes', 'Copier la sélection en Markdown'],
  },
  themes: { title: 'Deux thèmes, deux langues', line: 'Clair ou sombre, français ou anglais.' },
};

const EN: typeof FR = {
  label: 'And also',
  heading: 'And everything around it.',
  notes: {
    title: 'Notes',
    line: 'Written formatted, saved as Markdown.',
    heading: 'Certificate rotation',
    items: ['Issue from ACME', 'Reload nginx'],
  },
  lists: {
    title: 'Lists',
    line: 'To tick off, their progress on the card.',
    items: ['Freeze the branch', 'Run the CI', 'Publish the release', 'Tell the team'],
  },
  trash: {
    title: 'A trash',
    before: 'Thirty days to change your mind, and',
    after: 'right after a deletion.',
    toast: 'Note moved to the trash',
    undo: 'Undo',
  },
  versions: {
    title: 'Versions',
    line: 'The last twenty versions of a snippet, to read again and restore.',
    items: [
      ['Current version', 'just now'],
      ['Before formatting', '2 min ago'],
      ['Port changed', 'yesterday'],
    ],
  },
  attachments: {
    title: 'Attachments',
    line: 'Files and pasted images, encrypted with the note.',
    items: ['invoice-schema.png', 'api-contract.pdf'],
  },
  selection: {
    title: 'Multiple selection',
    line: 'Move, tag or delete twenty notes at once.',
    count: '20 selected',
    actions: ['Move to', 'Priority', '+ tag'],
  },
  transfer: {
    title: 'Import and export',
    before: 'A',
    after: ' archive, protected if you wish, or the selection copied as Markdown.',
    items: ['work-2026-10-14.devnotes', 'Copy the selection as Markdown'],
  },
  themes: { title: 'Two themes, two languages', line: 'Light or dark, French or English.' },
};

const TEXT: Readonly<Record<Lang, typeof FR>> = { fr: FR, en: EN };

/** Everything around the notes, a tile each. Two of them play once in view: a list ticks itself, a deletion counts down. */
@Component({
  selector: 'app-more-section',
  templateUrl: './more-section.html',
  styleUrl: './more-section.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class MoreSection {
  private readonly lang = inject(Language).lang;
  protected readonly text = computed(() => TEXT[this.lang()]);
  /** How many items of the list are ticked. */
  protected readonly ticked = signal(1);
  protected readonly countingDown = signal(false);
  private readonly list = viewChild.required<ElementRef<HTMLElement>>('list');
  private readonly trash = viewChild.required<ElementRef<HTMLElement>>('trash');

  constructor() {
    const destroyRef = inject(DestroyRef);
    afterNextRender(() => {
      if (!canObserve()) return;
      const list = this.list().nativeElement;
      const watch = new IntersectionObserver(
        (entries) => {
          for (const entry of entries) {
            if (!entry.isIntersecting) continue;
            watch.unobserve(entry.target);
            if (entry.target === list) void this.tick();
            else this.countingDown.set(true);
          }
        },
        { threshold: 0.7 },
      );
      watch.observe(list);
      watch.observe(this.trash().nativeElement);
      destroyRef.onDestroy(() => watch.disconnect());
    });
  }

  private async tick(): Promise<void> {
    const reduce = prefersReducedMotion();
    while (this.ticked() < this.text().lists.items.length) {
      if (!reduce) await sleep(650);
      this.ticked.update((count) => count + 1);
    }
  }
}
