import { RenderMode, ServerRoute } from '@angular/ssr';

/** Every page is written to disk at build time: a crawler reads the text without running a script. */
export const serverRoutes: ServerRoute[] = [{ path: '**', renderMode: RenderMode.Prerender }];
