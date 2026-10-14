import { ChangeDetectionStrategy, Component, computed, inject, signal } from '@angular/core';
import { takeUntilDestroyed } from '@angular/core/rxjs-interop';
import { NavigationEnd, Router, RouterLink, RouterLinkActive } from '@angular/router';
import { filter } from 'rxjs';

import { PageProgress, Tone } from '../core/page-progress';
import { ThemeStore } from '../core/theme.store';
import { Logo } from './logo';

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
  protected readonly menuOpen = signal(false);

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
