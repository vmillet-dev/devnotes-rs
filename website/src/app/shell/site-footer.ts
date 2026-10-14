import { ChangeDetectionStrategy, Component } from '@angular/core';
import { RouterLink } from '@angular/router';

import { CHANGELOG, LICENSE, REPOSITORY } from '../core/site';
import { Logo } from './logo';

@Component({
  selector: 'app-site-footer',
  imports: [RouterLink, Logo],
  template: `
    <footer class="site-foot">
      <div class="wrap">
        <div class="foot-brand">
          <span class="brand"><app-logo />DevNotes</span>
          <p>
            Logiciel libre et gratuit, sous licence GPL-3.0. Aucun compte, aucune télémétrie : vos notes ne
            quittent pas votre machine.
          </p>
        </div>
        <nav class="foot-links" aria-label="Pied de page">
          <a routerLink="/docs">Documentation</a>
          <a [href]="repository">Code source</a>
          <a [href]="changelog">Journal des modifications</a>
          <a [href]="license">Licence</a>
          <a routerLink="/docs/securite">Confidentialité</a>
        </nav>
      </div>
    </footer>
  `,
  styles: `
    .site-foot {
      border-top: 1px solid var(--line-soft);
      margin-top: clamp(64px, 10vw, 128px);
      padding-block: 40px 24px;
      color: var(--text-1);
      font-size: 14.5px;
    }

    .wrap {
      display: grid;
      grid-template-columns: minmax(0, 1.2fr) minmax(0, 1fr);
      gap: 24px 48px;
    }

    .foot-brand {
      display: grid;
      gap: 10px;
      max-width: 34em;
    }

    .brand {
      display: inline-flex;
      align-items: center;
      gap: 10px;
      font-weight: 720;
      font-size: 17px;
      color: var(--text-0);
    }

    .foot-links {
      display: flex;
      flex-wrap: wrap;
      gap: 6px 22px;
      align-content: start;
    }

    .foot-links a {
      text-underline-offset: 3px;
      padding-block: 4px;
    }

    @media (max-width: 700px) {
      .wrap {
        grid-template-columns: minmax(0, 1fr);
      }
    }
  `,
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class SiteFooter {
  protected readonly repository = REPOSITORY;
  protected readonly changelog = CHANGELOG;
  protected readonly license = LICENSE;
}
