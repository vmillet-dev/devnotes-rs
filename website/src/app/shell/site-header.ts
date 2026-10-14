import { ChangeDetectionStrategy, Component, computed, inject, signal } from '@angular/core';
import { takeUntilDestroyed } from '@angular/core/rxjs-interop';
import { NavigationEnd, Router, RouterLink, RouterLinkActive } from '@angular/router';
import { filter } from 'rxjs';

import { Lang, downloadPath, homePath } from '../core/lang';
import { Language } from '../core/language';
import { PageProgress, Tone } from '../core/page-progress';
import { REPOSITORY } from '../core/site';
import { ThemeStore } from '../core/theme.store';
import { docPath } from '../docs/doc-index';
import { Logo } from './logo';

const FR = {
  home: 'DevNotes, accueil',
  main: 'Principale',
  features: 'Fonctionnalités',
  docs: 'Documentation',
  download: 'Télécharger',
  toLight: 'Passer au thème clair',
  toDark: 'Passer au thème sombre',
  menu: 'Menu',
  source: 'Le code source sur GitHub',
  switchTo: 'Read this page in English',
};

const EN: typeof FR = {
  home: 'DevNotes, home',
  main: 'Main',
  features: 'Features',
  docs: 'Documentation',
  download: 'Download',
  toLight: 'Switch to the light theme',
  toDark: 'Switch to the dark theme',
  menu: 'Menu',
  source: 'Source code on GitHub',
  switchTo: 'Lire cette page en français',
};

const TEXT: Readonly<Record<Lang, typeof FR>> = { fr: FR, en: EN };

@Component({
  selector: 'app-site-header',
  imports: [RouterLink, RouterLinkActive, Logo],
  templateUrl: './site-header.html',
  styleUrl: './site-header.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class SiteHeader {
  protected readonly theme = inject(ThemeStore);
  protected readonly progress = inject(PageProgress);
  protected readonly language = inject(Language);
  protected readonly repository = REPOSITORY;
  protected readonly menuOpen = signal(false);

  protected readonly t = computed(() => TEXT[this.language.lang()]);
  protected readonly links = computed(() => {
    const lang = this.language.lang();
    return { home: homePath(lang), docs: docPath('overview', lang), download: downloadPath(lang) };
  });

  /** The section the visitor is reading, named beside the logo. */
  protected readonly here = computed(() => this.progress.segments()[this.progress.reading().current] ?? null);

  constructor() {
    inject(Router)
      .events.pipe(
        filter((event) => event instanceof NavigationEnd),
        takeUntilDestroyed(),
      )
      .subscribe(() => this.menuOpen.set(false));
  }

  protected rgb(tone: Tone): string {
    return `var(--${tone}-rgb)`;
  }

  protected toggleMenu(): void {
    this.menuOpen.update((open) => !open);
  }
}
