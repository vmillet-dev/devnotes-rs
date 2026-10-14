import { Tone } from '../core/page-progress';

export interface DocEntry {
  /** Its path under `/docs`, empty for the overview. */
  readonly slug: string;
  /** Its name in the sidebar and in the previous / next links. */
  readonly nav: string;
  readonly tone: Tone;
  /** The heading it is listed under in the sidebar. */
  readonly group: string;
}

/**
 * The documentation's pages, in reading order. Kept apart from their text so the routes
 * and the sidebar do not pull every page into the first download.
 */
export const DOC_INDEX: readonly DocEntry[] = [
  { slug: '', nav: 'Vue d’ensemble', tone: 'grey', group: 'Démarrer' },
  { slug: 'prise-en-main', nav: 'Prise en main', tone: 'amber', group: 'Démarrer' },
  { slug: 'ecrire', nav: 'Écrire', tone: 'green', group: 'Au quotidien' },
  { slug: 'ranger', nav: 'Ranger et retrouver', tone: 'blue', group: 'Au quotidien' },
  { slug: 'collage-rapide', nav: 'Collage rapide et raccourcis', tone: 'green', group: 'Au quotidien' },
  { slug: 'outils', nav: 'Les outils', tone: 'teal', group: 'Les outils et l’API' },
  { slug: 'client-api', nav: 'Le client API', tone: 'olive', group: 'Les outils et l’API' },
  { slug: 'securite', nav: 'Sécurité et sauvegardes', tone: 'pink', group: 'Vos données' },
  { slug: 'import-export', nav: 'Importer et exporter', tone: 'purple', group: 'Vos données' },
  { slug: 'preferences', nav: 'Préférences', tone: 'grey', group: 'Réglages et aide' },
  { slug: 'faq', nav: 'Questions fréquentes', tone: 'grey', group: 'Réglages et aide' },
];

/** The sidebar's groups, in reading order, each with its pages. */
export const DOC_GROUPS: readonly { readonly name: string; readonly entries: readonly DocEntry[] }[] = [
  ...new Set(DOC_INDEX.map(({ group }) => group)),
].map((name) => ({ name, entries: DOC_INDEX.filter((entry) => entry.group === name) }));

export function docPath(slug: string): string {
  return slug ? `/docs/${slug}` : '/docs';
}
