import { ChangeDetectionStrategy, Component, computed, inject } from '@angular/core';
import { RouterLink } from '@angular/router';

import { Lang, accessibilityPath, downloadPath } from '../core/lang';
import { Language } from '../core/language';
import { VERSION } from '../core/release';
import { CHANGELOG, LICENSE, REPOSITORY } from '../core/site';
import { docPath } from '../docs/doc-index';
import { Logo } from './logo';

const FR = {
  about:
    'Logiciel libre et gratuit, sous licence GPL-3.0. Aucun compte, aucune télémétrie : vos notes ne quittent pas votre machine.',
  nav: 'Pied de page',
  site: 'Le site',
  project: 'Le projet',
  version: 'Version',
  download: 'Télécharger',
  docs: 'Documentation',
  source: 'Code source',
  changelog: 'Journal des modifications',
  license: 'Licence',
  privacy: 'Confidentialité',
  accessibility: 'Accessibilité',
};

const EN: typeof FR = {
  about:
    'Free software, released under the GPL-3.0. No account, no telemetry: your notes never leave your machine.',
  nav: 'Footer',
  site: 'The site',
  project: 'The project',
  version: 'Version',
  download: 'Download',
  docs: 'Documentation',
  source: 'Source code',
  changelog: 'Changelog',
  license: 'License',
  privacy: 'Privacy',
  accessibility: 'Accessibility',
};

const TEXT: Readonly<Record<Lang, typeof FR>> = { fr: FR, en: EN };

@Component({
  selector: 'app-site-footer',
  imports: [RouterLink, Logo],
  template: `
    <footer class="site-foot sec">
      <div class="cells foot-grid">
        <div class="cell foot-brand">
          <span class="brand"><app-logo />DevNotes</span>
          <p>{{ t().about }}</p>
          <p class="version">{{ t().version }} {{ version }}</p>
        </div>
        <nav class="cell foot-links" [attr.aria-label]="t().nav">
          <div>
            <p class="h">{{ t().site }}</p>
            <a [routerLink]="links().download">{{ t().download }}</a>
            <a [routerLink]="links().docs">{{ t().docs }}</a>
            <a [routerLink]="links().privacy">{{ t().privacy }}</a>
            <a [routerLink]="links().accessibility">{{ t().accessibility }}</a>
          </div>
          <div>
            <p class="h">{{ t().project }}</p>
            <a [href]="repository">{{ t().source }}</a>
            <a [href]="changelog">{{ t().changelog }}</a>
            <a [href]="license">{{ t().license }}</a>
          </div>
        </nav>
      </div>
    </footer>
  `,
  styles: `
    .site-foot {
      color: var(--text-1);
      font-size: 14.5px;
    }

    .foot-grid {
      display: grid;
      grid-template-columns: minmax(0, 1.2fr) minmax(0, 1fr);
    }

    .cell {
      padding: 36px var(--pad) 40px;
    }

    .cell + .cell {
      border-left: 1px solid var(--line-soft);
    }

    .foot-brand {
      display: grid;
      align-content: start;
      gap: 12px;
    }

    .foot-brand p {
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

    .version {
      font: 500 12.5px var(--font-mono);
      color: var(--text-2);
    }

    .foot-links {
      display: grid;
      grid-template-columns: repeat(2, minmax(0, 1fr));
      gap: 24px;
    }

    .foot-links div {
      display: grid;
      align-content: start;
      gap: 4px;
    }

    .h {
      margin-bottom: 6px;
      font: 500 12.5px var(--font-mono);
      color: var(--text-2);
    }

    .foot-links a {
      justify-self: start;
      padding-block: 4px;
      text-underline-offset: 3px;
    }

    @media (max-width: 760px) {
      .foot-grid {
        grid-template-columns: minmax(0, 1fr);
      }

      .cell + .cell {
        border-left: 0;
        border-top: 1px solid var(--line-soft);
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
  protected readonly version = VERSION;
  protected readonly t = computed(() => TEXT[this.language.lang()]);
  protected readonly links = computed(() => {
    const lang = this.language.lang();
    return {
      download: downloadPath(lang),
      docs: docPath('overview', lang),
      privacy: docPath('safety', lang),
      accessibility: accessibilityPath(lang),
    };
  });
}
