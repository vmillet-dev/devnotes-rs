import { ChangeDetectionStrategy, Component, inject } from '@angular/core';
import { RouterLink } from '@angular/router';

import { Seo } from '../core/seo';

@Component({
  selector: 'app-not-found-page',
  imports: [RouterLink],
  template: `
    <section class="wrap lost" aria-labelledby="lost-h">
      <span class="chip">404</span>
      <h1 id="lost-h">Cette page n'existe pas.</h1>
      <p class="lede">
        L'adresse a peut-être changé. La documentation et le téléchargement sont toujours là.
      </p>
      <div class="cta-row">
        <a class="btn btn-primary" routerLink="/">Revenir à l'accueil</a>
        <a class="btn btn-ghost" routerLink="/docs">Lire la documentation</a>
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
  constructor() {
    inject(Seo).describe({
      title: 'Page introuvable · DevNotes',
      description:
        "Cette page n'existe pas. La documentation et le téléchargement de DevNotes sont toujours là.",
      path: '/404',
    });
  }
}
