import { ChangeDetectionStrategy, Component, computed, inject } from '@angular/core';
import { RouterLink } from '@angular/router';

import { Lang, downloadPath } from '../core/lang';
import { Language } from '../core/language';
import { CHANGELOG, LICENSE, REPOSITORY } from '../core/site';
import { docPath } from '../docs/doc-index';
import { Logo } from './logo';

const FR = {
  about:
    'Logiciel libre et gratuit, sous licence GPL-3.0. Aucun compte, aucune télémétrie : vos notes ne quittent pas votre machine.',
  nav: 'Pied de page',
  download: 'Télécharger',
  docs: 'Documentation',
  source: 'Code source',
  changelog: 'Journal des modifications',
  license: 'Licence',
  privacy: 'Confidentialité',
};

const EN: typeof FR = {
  about:
    'Free software, released under the GPL-3.0. No account, no telemetry: your notes never leave your machine.',
  nav: 'Footer',
  download: 'Download',
  docs: 'Documentation',
  source: 'Source code',
  changelog: 'Changelog',
  license: 'License',
  privacy: 'Privacy',
};

const TEXT: Readonly<Record<Lang, typeof FR>> = { fr: FR, en: EN };

@Component({
  selector: 'app-site-footer',
  imports: [RouterLink, Logo],
  template: `
    <footer class="site-foot">
      <div class="wrap">
        <div class="foot-brand">
          <span class="brand"><app-logo />DevNotes</span>
          <p>{{ t().about }}</p>
        </div>
        <nav class="foot-links" [attr.aria-label]="t().nav">
          <a [routerLink]="links().download">{{ t().download }}</a>
          <a [routerLink]="links().docs">{{ t().docs }}</a>
          <a [href]="repository">{{ t().source }}</a>
          <a [href]="changelog">{{ t().changelog }}</a>
          <a [href]="license">{{ t().license }}</a>
          <a [routerLink]="links().privacy">{{ t().privacy }}</a>
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
  private readonly language = inject(Language);
  protected readonly repository = REPOSITORY;
  protected readonly changelog = CHANGELOG;
  protected readonly license = LICENSE;
  protected readonly t = computed(() => TEXT[this.language.lang()]);
  protected readonly links = computed(() => {
    const lang = this.language.lang();
    return {
      download: downloadPath(lang),
      docs: docPath('overview', lang),
      privacy: docPath('safety', lang),
    };
  });
}
