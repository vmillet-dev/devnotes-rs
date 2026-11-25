import { Route, Routes } from '@angular/router';

import { Lang } from './core/lang';
import { SITE_PAGES, SitePage } from './core/pages';

const LOADERS: Readonly<Record<SitePage['kind'], NonNullable<Route['loadComponent']>>> = {
  home: () => import('./home/home-page').then((m) => m.HomePage),
  download: () => import('./download/download-page').then((m) => m.DownloadPage),
  docs: () => import('./docs/docs-page').then((m) => m.DocsPage),
};

const notFound = () => import('./not-found/not-found-page').then((m) => m.NotFoundPage);

/** One route per page and per language, each written to its own `index.html` at build time. */
export const routes: Routes = [
  ...(['fr', 'en'] as const satisfies readonly Lang[]).flatMap((lang) =>
    SITE_PAGES.map((page) => ({
      path: page.paths[lang].slice(1),
      loadComponent: LOADERS[page.kind],
      data: { doc: page.doc },
    })),
  ),
  { path: '404', loadComponent: notFound },
  { path: '**', loadComponent: notFound },
];
