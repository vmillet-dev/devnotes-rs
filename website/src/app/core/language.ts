import { PlatformLocation } from '@angular/common';
import { Injectable, computed, inject } from '@angular/core';
import { toSignal } from '@angular/core/rxjs-interop';
import { NavigationEnd, ResolveEnd, Router } from '@angular/router';
import { filter, map } from 'rxjs';

import { OTHER, langOf } from './lang';
import { translatedPath } from './pages';

const withoutFragment = (url: string) => url.split('#')[0] ?? '/';

/** The language of the page on screen, read from its address, and the way to the other one. */
@Injectable({ providedIn: 'root' })
export class Language {
  private readonly router = inject(Router);

  /**
   * The page's own path, without its fragment. Taken at `ResolveEnd`, not `NavigationEnd`: a page
   * is built in between, and its constructor already writes its title in its language.
   */
  readonly path = toSignal(
    this.router.events.pipe(
      filter((event) => event instanceof ResolveEnd || event instanceof NavigationEnd),
      map((event) => withoutFragment(event.urlAfterRedirects)),
    ),
    { initialValue: inject(PlatformLocation).pathname || '/' },
  );
  readonly lang = computed(() => langOf(this.path()));
  readonly other = computed(() => OTHER[this.lang()]);
  /** The same page in the other language. */
  readonly switchTo = computed(() => translatedPath(this.path(), this.other()));
}
