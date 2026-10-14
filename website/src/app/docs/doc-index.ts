import { Tone } from '../core/page-progress';

export interface DocEntry {
  /** Its path under `/docs`, empty for the overview. */
  readonly slug: string;
  /** Its name in the sidebar and in the previous / next links. */
  readonly nav: string;
  readonly tone: Tone;
}

/**
 * The documentation's pages, in reading order. Kept apart from their text so the routes
 * and the sidebar do not pull every page into the first download.
 */
export const DOC_INDEX: readonly DocEntry[] = [
  { slug: '', nav: 'Vue d’ensemble', tone: 'grey' },
  { slug: 'prise-en-main', nav: 'Prise en main', tone: 'amber' },
  { slug: 'ecrire', nav: 'Écrire', tone: 'green' },
  { slug: 'ranger', nav: 'Ranger et retrouver', tone: 'blue' },
  { slug: 'collage-rapide', nav: 'Collage rapide et raccourcis', tone: 'green' },
  { slug: 'outils', nav: 'Les outils', tone: 'teal' },
  { slug: 'client-api', nav: 'Le client API', tone: 'olive' },
  { slug: 'securite', nav: 'Sécurité et sauvegardes', tone: 'pink' },
  { slug: 'import-export', nav: 'Importer et exporter', tone: 'purple' },
  { slug: 'preferences', nav: 'Préférences', tone: 'grey' },
  { slug: 'faq', nav: 'Questions fréquentes', tone: 'grey' },
];

export function docPath(slug: string): string {
  return slug ? `/docs/${slug}` : '/docs';
}
