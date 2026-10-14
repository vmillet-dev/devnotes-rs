import { ChangeDetectionStrategy, Component, computed, inject } from '@angular/core';
import { RouterLink } from '@angular/router';

import { Lang, homePath } from '../core/lang';
import { Language } from '../core/language';
import { Seo } from '../core/seo';
import { docPath } from '../docs/doc-index';

const FR = {
  title: 'Page introuvable · DevNotes',
  description: "Cette page n'existe pas. La documentation et le téléchargement de DevNotes sont toujours là.",
  heading: "Cette page n'existe pas.",
  lede: "L'adresse a peut-être changé. La documentation et le téléchargement sont toujours là.",
  home: "Revenir à l'accueil",
  docs: 'Lire la documentation',
};

const EN: typeof FR = {
  title: 'Page not found · DevNotes',
  description: 'This page does not exist. The DevNotes documentation and download are still there.',
  heading: 'This page does not exist.',
  lede: 'The address may have changed. The documentation and the download are still there.',
  home: 'Back to the home page',
  docs: 'Read the documentation',
};

const TEXT: Readonly<Record<Lang, typeof FR>> = { fr: FR, en: EN };

@Component({
  selector: 'app-not-found-page',
  imports: [RouterLink],
  template: `
    <section class="wrap lost" aria-labelledby="lost-h">
      <span class="chip">404</span>
      <h1 id="lost-h">{{ text().heading }}</h1>
      <p class="lede">{{ text().lede }}</p>
      <div class="cta-row">
        <a class="btn btn-primary" [routerLink]="home()">{{ text().home }}</a>
        <a class="btn btn-ghost" [routerLink]="docs()">{{ text().docs }}</a>
      </div>
    </section>
  `,
  styles: `
    .lost {
      display: grid;
      gap: 22px;
      justify-items: start;
      padding-block: clamp(64px, 12vw, 160px) 0;
    }

    .chip {
      padding: 4px 10px;
      border-radius: 999px;
      border: 1px solid var(--line);
      color: var(--text-2);
      font: 600 13px var(--font-mono);
    }
  `,
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class NotFoundPage {
  private readonly lang = inject(Language).lang;
  protected readonly text = computed(() => TEXT[this.lang()]);
  protected readonly home = computed(() => homePath(this.lang()));
  protected readonly docs = computed(() => docPath('overview', this.lang()));

  constructor() {
    const lang = this.lang();
    const { title, description } = TEXT[lang];
    inject(Seo).describe({ title, description, path: '/404', lang, translated: false });
  }
}
