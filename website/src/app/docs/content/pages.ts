import { DocPage } from '../doc.model';
import { API_CLIENT } from './api';
import { QUICK_PASTE } from './paste';
import { SAFETY } from './safety';
import { FAQ, SETTINGS, TRANSFER } from './settings';
import { GETTING_STARTED, OVERVIEW } from './start';
import { TOOLS } from './tools';
import { ORGANISING, WRITING } from './writing';

/** The version these pages were last checked against, by hand, in the application itself. */
export const CHECKED_ON = '0.12';

/** Every page's text, in no particular order: `DOC_INDEX` says which comes first. */
export const DOC_PAGES: readonly DocPage[] = [
  OVERVIEW,
  GETTING_STARTED,
  WRITING,
  ORGANISING,
  QUICK_PASTE,
  TOOLS,
  API_CLIENT,
  SAFETY,
  TRANSFER,
  SETTINGS,
  FAQ,
];
