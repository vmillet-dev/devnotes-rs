import { Injectable, InjectionToken, inject } from '@angular/core';
import { openUrl } from '@tauri-apps/plugin-opener';

/** A token: a spec really opening the browser would leave windows behind. */
export interface ExternalLinkAdapter {
  open(url: string): Promise<void>;
}

export const EXTERNAL_LINK_ADAPTER = new InjectionToken<ExternalLinkAdapter>('EXTERNAL_LINK_ADAPTER', {
  providedIn: 'root',
  factory: () => ({ open: (url) => openUrl(url) }),
});

/** A link written in a note, opened in the browser. */
@Injectable({ providedIn: 'root' })
export class ExternalLinksService {
  private readonly adapter = inject(EXTERNAL_LINK_ADAPTER);

  /** The web only: a note is anyone's text, and a `file:` link is a program to run. */
  async open(url: string): Promise<boolean> {
    if (!/^https?:\/\//i.test(url)) return false;

    await this.adapter.open(url);
    return true;
  }
}
