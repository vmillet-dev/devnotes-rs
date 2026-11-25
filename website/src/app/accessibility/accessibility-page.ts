import { ChangeDetectionStrategy, Component, computed, inject } from '@angular/core';

import { Lang, accessibilityPath } from '../core/lang';
import { Language } from '../core/language';
import { Seo } from '../core/seo';
import { REPOSITORY } from '../core/site';

const FR = {
  title: 'Accessibilité · DevNotes',
  description:
    'Ce que devnotes.fr vise en matière d’accessibilité, ce qui est vérifié, ce qui ne l’est pas encore, et comment signaler un problème.',
  heading: 'Accessibilité',
  lede: 'Ce site vise le niveau AA des WCAG 2.2. Voici ce qui est vérifié, ce qui ne l’est pas encore, et comment signaler un problème.',
  done: 'Ce qui est vérifié',
  checks: [
    'Tout se fait au clavier, avec un lien pour aller droit au contenu et un contour visible sur l’élément actif.',
    'Le texte garde un contraste d’au moins 4,5:1 dans les deux thèmes, et les contours des commandes d’au moins 3:1.',
    'Chaque page a un titre, des repères, des tableaux titrés et des commandes nommées ; une cible mesure au moins 24 × 24 pixels.',
    'Les pages tiennent à 320 pixels de large et à 200 % de taille de texte, sans défilement horizontal.',
    'Les animations s’arrêtent si votre système demande moins de mouvement.',
  ],
  limits: 'Ce qui ne l’est pas encore',
  missing: [
    'Les pages n’ont pas encore été relues avec un lecteur d’écran.',
    'Les scènes de la page d’accueil redessinent l’application pour l’illustrer : elles sont masquées aux lecteurs d’écran, et le texte à côté dit la même chose.',
  ],
  report: 'Signaler un problème',
  reportBefore: 'Ouvrez un ticket sur ',
  reportLink: 'le dépôt GitHub',
  reportAfter: ' en disant quelle page, ce que vous utilisez et ce qui vous a bloqué.',
};

const EN: typeof FR = {
  title: 'Accessibility · DevNotes',
  description:
    'What devnotes.fr aims at for accessibility, what is checked, what is not yet, and how to report a problem.',
  heading: 'Accessibility',
  lede: 'This site aims at WCAG 2.2, level AA. Here is what is checked, what is not yet, and how to report a problem.',
  done: 'What is checked',
  checks: [
    'Everything works from the keyboard, with a link straight to the content and a visible outline on the active element.',
    'Text keeps a contrast of at least 4.5:1 in both themes, and the outlines of controls at least 3:1.',
    'Each page has a title, landmarks, titled tables and named controls; a target is at least 24 × 24 pixels.',
    'Pages hold at 320 pixels wide and at 200% text size, without sideways scrolling.',
    'Animations stop if your system asks for less motion.',
  ],
  limits: 'What is not yet',
  missing: [
    'The pages have not yet been read with a screen reader.',
    'The scenes of the home page redraw the application to illustrate it: they are hidden from screen readers, and the text beside them says the same.',
  ],
  report: 'Reporting a problem',
  reportBefore: 'Open an issue on ',
  reportLink: 'the GitHub repository',
  reportAfter: ', saying which page, what you use and what stood in your way.',
};

const TEXT: Readonly<Record<Lang, typeof FR>> = { fr: FR, en: EN };

@Component({
  selector: 'app-accessibility-page',
  template: `
    <section class="wrap statement" aria-labelledby="statement-h">
      <h1 id="statement-h">{{ text().heading }}</h1>
      <p class="lede">{{ text().lede }}</p>
      <h2>{{ text().done }}</h2>
      <ul>
        @for (check of text().checks; track $index) {
          <li>{{ check }}</li>
        }
      </ul>
      <h2>{{ text().limits }}</h2>
      <ul>
        @for (limit of text().missing; track $index) {
          <li>{{ limit }}</li>
        }
      </ul>
      <h2>{{ text().report }}</h2>
      <p>
        {{ text().reportBefore }}<a [href]="issues">{{ text().reportLink }}</a
        >{{ text().reportAfter }}
      </p>
    </section>
  `,
  styles: `
    .statement {
      display: grid;
      gap: 18px;
      padding-block: clamp(48px, 8vw, 96px) 0;
    }

    .statement > * {
      max-width: 44rem;
    }

    h2 {
      margin-top: 22px;
      font-size: 1.5rem;
      letter-spacing: -0.02em;
    }

    ul {
      display: grid;
      gap: 10px;
      padding-left: 1.2em;
      list-style: disc;
    }

    li,
    p {
      color: var(--text-1);
      line-height: 1.65;
    }

    a {
      color: var(--amber-text);
      text-underline-offset: 3px;
    }
  `,
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class AccessibilityPage {
  private readonly lang = inject(Language).lang;
  protected readonly text = computed(() => TEXT[this.lang()]);
  protected readonly issues = `${REPOSITORY}/issues`;

  constructor() {
    const lang = this.lang();
    const { title, description } = TEXT[lang];
    inject(Seo).describe({ title, description, path: accessibilityPath(lang), lang });
  }
}
