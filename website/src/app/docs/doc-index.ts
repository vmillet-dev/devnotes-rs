import index from '../../../../docs/content/index.json';
import { Lang } from '../core/lang';

type PerLang = Readonly<Record<Lang, string>>;

/** A page's colour in the sidebar, as the tokens name its RGB triplet (`--green-rgb`). */
export type Tone = 'green' | 'blue' | 'purple' | 'teal' | 'olive' | 'pink' | 'grey' | 'amber';

/** `docs/content/index.json`, as it is written there. */
interface DocsIndex {
  readonly groups: readonly {
    readonly name: PerLang;
    readonly pages: readonly {
      readonly id: string;
      readonly colour: string;
      readonly path: PerLang;
      readonly name: PerLang;
    }[];
  }[];
}

export interface DocEntry {
  /** The page's name in code, the same in every language. */
  readonly id: string;
  /** Its path under `/docs`, per language; empty for the overview. */
  readonly slug: PerLang;
  /** Its name in the sidebar and in the previous / next links. */
  readonly nav: PerLang;
  readonly tone: Tone;
  /** The heading it is listed under in the sidebar. */
  readonly group: PerLang;
}

const INDEX: DocsIndex = index;

/**
 * The documentation's pages, in reading order. Kept apart from their text so the routes
 * and the sidebar do not pull every page into the first download.
 */
export const DOC_INDEX: readonly DocEntry[] = INDEX.groups.flatMap((group) =>
  group.pages.map(({ id, colour, path, name }) => ({
    id,
    slug: path,
    nav: name,
    // `scripts/docs.mjs` refuses a colour the palette has not.
    tone: colour as Tone,
    group: group.name,
  })),
);

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
