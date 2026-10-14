import {
  ChangeDetectionStrategy,
  Component,
  DOCUMENT,
  DestroyRef,
  afterNextRender,
  inject,
} from '@angular/core';
import { RouterLink } from '@angular/router';

import { clamp, field } from '../core/browser';
import { PageProgress, ProgressSegment } from '../core/page-progress';
import { ScrollFrame } from '../core/scroll-frame';
import { Seo } from '../core/seo';
import { VERSION } from '../core/release';
import { REPOSITORY, absolute } from '../core/site';
import { Feature } from './feature';
import { HeroBoard } from './hero-board';
import { MoreSection } from './more-section';
import { ApiScene } from './scenes/api-scene';
import { CodeScene } from './scenes/code-scene';
import { OrganiseScene } from './scenes/organise-scene';
import { SnippetsScene } from './scenes/snippets-scene';
import { ToolsScene } from './scenes/tools-scene';
import { VaultScene } from './scenes/vault-scene';

/** The sections the line under the header follows, in page order. */
const SECTIONS: readonly ProgressSegment[] = [
  { anchor: 'snippets', name: 'Snippets', tone: 'green' },
  { anchor: 'ranger', name: 'Ranger', tone: 'blue' },
  { anchor: 'code', name: 'Code', tone: 'purple' },
  { anchor: 'outils', name: 'Outils', tone: 'teal' },
  { anchor: 'client-api', name: 'Client API', tone: 'olive' },
  { anchor: 'chiffrement', name: 'Chiffrement', tone: 'pink' },
  { anchor: 'et-aussi', name: 'Et aussi', tone: 'grey' },
];

@Component({
  selector: 'app-home-page',
  imports: [
    RouterLink,
    HeroBoard,
    Feature,
    SnippetsScene,
    OrganiseScene,
    CodeScene,
    ApiScene,
    VaultScene,
    ToolsScene,
    MoreSection,
  ],
  templateUrl: './home-page.html',
  styleUrl: './home-page.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class HomePage {
  protected readonly field = field;
  protected readonly version = VERSION;

  constructor() {
    const progress = inject(PageProgress);
    const document = inject(DOCUMENT);
    const frame = inject(ScrollFrame);
    const destroyRef = inject(DestroyRef);

    inject(Seo).describe({
      title: 'DevNotes : snippets, outils et client API pour développeurs',
      description:
        'Une application de bureau pour développeurs : snippets à champs, notes rangées en tableau, outils hors ligne et client API, chiffrés sur votre machine.',
      path: '/',
      data: {
        '@type': 'SoftwareApplication',
        name: 'DevNotes',
        applicationCategory: 'DeveloperApplication',
        operatingSystem: 'Windows, Linux',
        url: absolute('/'),
        softwareVersion: VERSION,
        downloadUrl: absolute('/telecharger'),
        license: 'https://www.gnu.org/licenses/gpl-3.0.html',
        codeRepository: REPOSITORY,
        offers: { '@type': 'Offer', price: '0', priceCurrency: 'EUR' },
      },
    });

    progress.show(SECTIONS);
    destroyRef.onDestroy(() => progress.clear());

    afterNextRender(() => {
      const sections = SECTIONS.map(({ anchor }) => document.getElementById(anchor));
      const stop = frame.watch(() => {
        // Read at the middle of the screen; the gap above a section is not the section yet.
        const probe = innerHeight * 0.5;
        let current = -1;
        const weights: number[] = [];
        const fills: number[] = [];
        sections.forEach((section, i) => {
          if (!section) {
            weights.push(1);
            fills.push(0);
            return;
          }
          const box = section.getBoundingClientRect();
          const start = (section.firstElementChild ?? section).getBoundingClientRect().top;
          weights.push(Math.round(box.height));
          fills.push(Number(clamp((probe - start) / Math.max(1, box.bottom - start)).toFixed(3)));
          if (start <= probe) current = i;
        });
        progress.report({ weights, fills, current });
      });
      destroyRef.onDestroy(stop);
    });
  }
}
