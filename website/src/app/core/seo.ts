import { DOCUMENT, Injectable, inject } from '@angular/core';
import { Meta, Title } from '@angular/platform-browser';

import { LOCALES, Lang, OTHER } from './lang';
import { translatedPath } from './pages';
import { absolute } from './site';

export interface PageMeta {
  readonly title: string;
  readonly description: string;
  /** The page's path on the site, `/` or `/en/docs/…`. */
  readonly path: string;
  readonly lang: Lang;
  /** False for a page that has no twin in the other language. */
  readonly translated?: boolean;
  /** Structured data for what the page shows, written as one JSON-LD script. */
  readonly data?: object;
}

const SITE_NAME = 'DevNotes';

/**
 * What a crawler and a link preview read, set by each page while it renders — at build
 * time, so it is in the HTML written to disk.
 */
@Injectable({ providedIn: 'root' })
export class Seo {
  private readonly document = inject(DOCUMENT);
  private readonly title = inject(Title);
  private readonly meta = inject(Meta);

  describe(page: PageMeta): void {
    const url = absolute(page.path);
    const other = OTHER[page.lang];

    this.document.documentElement.setAttribute('lang', page.lang);
    this.title.setTitle(page.title);
    this.meta.updateTag({ name: 'description', content: page.description });
    this.meta.updateTag({ property: 'og:type', content: 'website' });
    this.meta.updateTag({ property: 'og:site_name', content: SITE_NAME });
    this.meta.updateTag({ property: 'og:locale', content: LOCALES[page.lang].og });
    this.meta.updateTag({ property: 'og:locale:alternate', content: LOCALES[other].og });
    this.meta.updateTag({ property: 'og:title', content: page.title });
    this.meta.updateTag({ property: 'og:description', content: page.description });
    this.meta.updateTag({ property: 'og:url', content: url });
    this.meta.updateTag({ name: 'twitter:card', content: 'summary' });

    this.link('canonical').setAttribute('href', url);
    this.alternates(page);
    this.structuredData(page.data);
  }

  private link(rel: string): HTMLLinkElement {
    const head = this.document.head;
    let link = head.querySelector<HTMLLinkElement>(`link[rel="${rel}"]`);
    if (!link) {
      link = this.document.createElement('link');
      link.setAttribute('rel', rel);
      head.appendChild(link);
    }
    return link;
  }

  /** Both languages, and the French one for a visitor whose language is neither. */
  private alternates(page: PageMeta): void {
    const head = this.document.head;
    head.querySelectorAll('link[rel="alternate"][hreflang]').forEach((link) => link.remove());
    if (page.translated === false) return;

    const pairs: [string, string][] = [
      ['fr', translatedPath(page.path, 'fr')],
      ['en', translatedPath(page.path, 'en')],
      ['x-default', translatedPath(page.path, 'fr')],
    ];
    for (const [hreflang, path] of pairs) {
      const link = this.document.createElement('link');
      link.setAttribute('rel', 'alternate');
      link.setAttribute('hreflang', hreflang);
      link.setAttribute('href', absolute(path));
      head.appendChild(link);
    }
  }

  private structuredData(data: object | undefined): void {
    const head = this.document.head;
    head.querySelector('script[type="application/ld+json"]')?.remove();
    if (!data) return;

    const script = this.document.createElement('script');
    script.setAttribute('type', 'application/ld+json');
    script.textContent = JSON.stringify({ '@context': 'https://schema.org', ...data });
    head.appendChild(script);
  }
}
