import { Routes } from '@angular/router';

import { DOC_INDEX } from './docs/doc-index';

const docs = () => import('./docs/docs-page').then((m) => m.DocsPage);

/** One route per page, each written to its own `index.html` at build time. */
export const routes: Routes = [
  { path: '', loadComponent: () => import('./home/home-page').then((m) => m.HomePage) },
  ...DOC_INDEX.map(({ slug }) => ({
    path: slug ? `docs/${slug}` : 'docs',
    loadComponent: docs,
    data: { slug },
  })),
  { path: '404', loadComponent: () => import('./not-found/not-found-page').then((m) => m.NotFoundPage) },
  { path: '**', loadComponent: () => import('./not-found/not-found-page').then((m) => m.NotFoundPage) },
];
