import { DOC_INDEX, docPath } from '../docs/doc-index';
import { Lang, downloadPath, homePath } from './lang';

export type PageKind = 'home' | 'download' | 'docs';

/** Every page of the site, with its address in each language: the routes and the language switch read it. */
export interface SitePage {
  readonly kind: PageKind;
  /** For a documentation page, its id in `DOC_INDEX`. */
  readonly doc: string;
  readonly paths: Readonly<Record<Lang, string>>;
}

export const SITE_PAGES: readonly SitePage[] = [
  { kind: 'home', doc: '', paths: { fr: homePath('fr'), en: homePath('en') } },
  { kind: 'download', doc: '', paths: { fr: downloadPath('fr'), en: downloadPath('en') } },
  ...DOC_INDEX.map(({ id }) => ({
    kind: 'docs' as const,
    doc: id,
    paths: { fr: docPath(id, 'fr'), en: docPath(id, 'en') },
  })),
];

/** The same page in another language; the home page when the address is not one of the site's. */
export function translatedPath(path: string, to: Lang): string {
  const clean = path.split(/[?#]/)[0]?.replace(/(.)\/$/, '$1') ?? '/';
  const page = SITE_PAGES.find(({ paths }) => paths.fr === clean || paths.en === clean);
  return page ? page.paths[to] : homePath(to);
}
