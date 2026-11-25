import { Lang } from '../core/lang';
import { Tone } from '../core/page-progress';

export interface DocEntry {
  /** The page's name in code, the same in every language. */
  readonly id: string;
  /** Its path under `/docs`, per language; empty for the overview. */
  readonly slug: Readonly<Record<Lang, string>>;
  /** Its name in the sidebar and in the previous / next links. */
  readonly nav: Readonly<Record<Lang, string>>;
  readonly tone: Tone;
  /** The heading it is listed under in the sidebar. */
  readonly group: Readonly<Record<Lang, string>>;
}

const START = { fr: 'Démarrer', en: 'Getting started' };
const DAILY = { fr: 'Au quotidien', en: 'Every day' };
const TOOLS = { fr: 'Les outils et l’API', en: 'Tools and the API' };
const DATA = { fr: 'Vos données', en: 'Your data' };
const HELP = { fr: 'Réglages et aide', en: 'Settings and help' };

/**
 * The documentation's pages, in reading order. Kept apart from their text so the routes
 * and the sidebar do not pull every page into the first download.
 */
export const DOC_INDEX: readonly DocEntry[] = [
  {
    id: 'overview',
    slug: { fr: '', en: '' },
    nav: { fr: 'Vue d’ensemble', en: 'Overview' },
    tone: 'grey',
    group: START,
  },
  {
    id: 'start',
    slug: { fr: 'prise-en-main', en: 'getting-started' },
    nav: { fr: 'Prise en main', en: 'Getting started' },
    tone: 'amber',
    group: START,
  },
  {
    id: 'writing',
    slug: { fr: 'ecrire', en: 'writing' },
    nav: { fr: 'Écrire', en: 'Writing' },
    tone: 'green',
    group: DAILY,
  },
  {
    id: 'organising',
    slug: { fr: 'ranger', en: 'organising' },
    nav: { fr: 'Ranger et retrouver', en: 'Organising and finding' },
    tone: 'blue',
    group: DAILY,
  },
  {
    id: 'paste',
    slug: { fr: 'collage-rapide', en: 'quick-paste' },
    nav: { fr: 'Collage rapide et raccourcis', en: 'Quick paste and shortcuts' },
    tone: 'green',
    group: DAILY,
  },
  {
    id: 'tools',
    slug: { fr: 'outils', en: 'tools' },
    nav: { fr: 'Les outils', en: 'The tools' },
    tone: 'teal',
    group: TOOLS,
  },
  {
    id: 'api',
    slug: { fr: 'client-api', en: 'api-client' },
    nav: { fr: 'Le client API', en: 'The API client' },
    tone: 'olive',
    group: TOOLS,
  },
  {
    id: 'safety',
    slug: { fr: 'securite', en: 'security' },
    nav: { fr: 'Sécurité et sauvegardes', en: 'Security and backups' },
    tone: 'pink',
    group: DATA,
  },
  {
    id: 'transfer',
    slug: { fr: 'import-export', en: 'import-export' },
    nav: { fr: 'Importer et exporter', en: 'Import and export' },
    tone: 'purple',
    group: DATA,
  },
  {
    id: 'settings',
    slug: { fr: 'preferences', en: 'settings' },
    nav: { fr: 'Préférences', en: 'Settings' },
    tone: 'grey',
    group: HELP,
  },
  {
    id: 'faq',
    slug: { fr: 'faq', en: 'faq' },
    nav: { fr: 'Questions fréquentes', en: 'Questions' },
    tone: 'grey',
    group: HELP,
  },
];

export interface DocGroup {
  readonly name: string;
  readonly entries: readonly DocEntry[];
}

/** The sidebar's groups in a language, in reading order, each with its pages. */
export function docGroups(lang: Lang): readonly DocGroup[] {
  const names = [...new Set(DOC_INDEX.map(({ group }) => group[lang]))];
  return names.map((name) => ({ name, entries: DOC_INDEX.filter((entry) => entry.group[lang] === name) }));
}

/** A documentation page's address in a language. */
export function docPath(id: string, lang: Lang): string {
  const entry = DOC_INDEX.find((each) => each.id === id);
  const base = lang === 'fr' ? '/docs' : '/en/docs';
  return entry?.slug[lang] ? `${base}/${entry.slug[lang]}` : base;
}
