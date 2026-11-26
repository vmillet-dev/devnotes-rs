import { ChangeDetectionStrategy, Component, computed, inject } from '@angular/core';
import { RouterLink } from '@angular/router';

import { Lang, homePath } from '../core/lang';
import { Language } from '../core/language';
import { Seo } from '../core/seo';
import { docPath } from '../docs/doc-index';

const FR = {
  title: 'Page introuvable · DevNotes',
  label: 'Page introuvable',
  description: "Cette page n'existe pas. La documentation et le téléchargement de DevNotes sont toujours là.",
  heading: "Cette page n'existe pas.",
  lede: "L'adresse a peut-être changé. La documentation et le téléchargement sont toujours là.",
  home: "Revenir à l'accueil",
  docs: 'Lire la documentation',
};

const EN: typeof FR = {
  title: 'Page not found · DevNotes',
  label: 'Page not found',
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
    <section class="sec" aria-labelledby="lost-h">
      <div class="sec-h col">
        <span><b>404</b> / {{ text().label }}</span>
      </div>
      <div class="lost col">
        <h1 id="lost-h">{{ text().heading }}</h1>
        <p class="lede">{{ text().lede }}</p>
        <div class="cta-row">
          <a class="btn btn-primary" [routerLink]="home()">{{ text().home }}</a>
          <a class="btn btn-ghost" [routerLink]="docs()">{{ text().docs }}</a>
        </div>
      </div>
    </section>
  `,
  styles: `
    .lost {
      display: grid;
      gap: 22px;
      justify-items: start;
      padding-block: clamp(56px, 10vw, 128px);
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
