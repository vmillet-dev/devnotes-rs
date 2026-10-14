import { DOCUMENT, Injectable, inject } from '@angular/core';
import { Meta, Title } from '@angular/platform-browser';

import { absolute } from './site';

export interface PageMeta {
  readonly title: string;
  readonly description: string;
  /** The page's path on the site, `/` or `/docs/…`. */
  readonly path: string;
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

    this.title.setTitle(page.title);
    this.meta.updateTag({ name: 'description', content: page.description });
    this.meta.updateTag({ property: 'og:type', content: 'website' });
    this.meta.updateTag({ property: 'og:site_name', content: SITE_NAME });
    this.meta.updateTag({ property: 'og:locale', content: 'fr_FR' });
    this.meta.updateTag({ property: 'og:title', content: page.title });
    this.meta.updateTag({ property: 'og:description', content: page.description });
    this.meta.updateTag({ property: 'og:url', content: url });
    this.meta.updateTag({ name: 'twitter:card', content: 'summary' });

    this.link('canonical').setAttribute('href', url);
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
