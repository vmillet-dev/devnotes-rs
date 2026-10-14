import { ChangeDetectionStrategy, Component, afterNextRender, inject } from '@angular/core';
import { RouterOutlet } from '@angular/router';

import { Language } from './core/language';
import { ThemeStore } from './core/theme.store';
import { SiteFooter } from './shell/site-footer';
import { SiteHeader } from './shell/site-header';

@Component({
  selector: 'app-root',
  imports: [RouterOutlet, SiteHeader, SiteFooter],
  template: `
    <a class="skip-link" [href]="language.path() + '#contenu'">{{
      language.lang() === 'fr' ? 'Aller au contenu' : 'Skip to the content'
    }}</a>
    <app-site-header />
    <main id="contenu" tabindex="-1">
      <router-outlet />
    </main>
    <app-site-footer />
  `,
  styles: `
    .skip-link {
      position: absolute;
      left: 16px;
      top: -200px;
      z-index: 60;
      padding: 10px 14px;
      border-radius: 8px;
      background: var(--amber-fill);
      color: var(--amber-ink);
      font-weight: 650;
      text-decoration: none;
    }

    .skip-link:focus {
      top: 12px;
    }

    main:focus {
      outline: none;
    }
  `,
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class App {
  /**
   * The page's own path: under `<base href="/">` a bare `#contenu` would lead to the home
   * page's anchor from every other page.
   */
  protected readonly language = inject(Language);

  constructor() {
    const theme = inject(ThemeStore);
    afterNextRender(() => theme.connect());
  }
}
