import { Lang } from '../../core/lang';
import { DocPage } from '../doc.model';
import * as EN from './en';
import * as FR from './fr';

/** The version these pages were last checked against, by hand, in the application itself. */
export const CHECKED_ON = '0.12';

const inOrder = (pages: typeof FR): readonly DocPage[] => [
  pages.OVERVIEW,
  pages.GETTING_STARTED,
  pages.WRITING,
  pages.ORGANISING,
  pages.QUICK_PASTE,
  pages.TOOLS,
  pages.API_CLIENT,
  pages.SAFETY,
  pages.TRANSFER,
  pages.SETTINGS,
  pages.FAQ,
];

/** Every page's text, per language: `DOC_INDEX` says in which order they are read. */
export const DOC_PAGES: Readonly<Record<Lang, readonly DocPage[]>> = { fr: inOrder(FR), en: inOrder(EN) };
