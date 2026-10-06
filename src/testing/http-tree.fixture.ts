import { HttpMethod, HttpNode, HttpTree, RequestKind } from '@core/model/http.model';

export const request = (
  id: string,
  collectionId: string,
  folderId: string | null = null,
  method: HttpMethod = 'GET',
  kind: RequestKind = 'http',
): HttpNode => ({
  kind: 'request',
  request: { id, collectionId, folderId, name: id, kind, method, position: 0 },
});

export const folder = (
  id: string,
  collectionId: string,
  children: HttpNode[] = [],
  parentId: string | null = null,
): HttpNode => ({
  kind: 'folder',
  folder: { id, collectionId, parentId, name: id, position: 0 },
  children,
});

export const collection = (id: string, children: HttpNode[] = []) => ({
  collection: { id, name: id, position: 0, createdAt: '2026-10-07T00:00:00.000Z' },
  children,
});

/**
 * API: Auth[Login, Logout], Factures[Archives[Old]], Health (POST) ; Catalogue: Produits (QUERY) ;
 * Flux: Live (WS).
 */
export function sampleTree(): HttpTree {
  return {
    collections: [
      collection('API', [
        folder('Auth', 'API', [request('Login', 'API', 'Auth'), request('Logout', 'API', 'Auth')]),
        folder('Factures', 'API', [
          folder('Archives', 'API', [request('Old', 'API', 'Archives')], 'Factures'),
        ]),
        request('Health', 'API', null, 'POST'),
      ]),
      collection('Catalogue', [request('Produits', 'Catalogue', null, 'POST', 'graphql')]),
      collection('Flux', [request('Live', 'Flux', null, 'GET', 'websocket')]),
    ],
  };
}
