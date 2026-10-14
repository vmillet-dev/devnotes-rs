import {
  ChangeDetectionStrategy,
  Component,
  ElementRef,
  afterNextRender,
  signal,
  viewChildren,
} from '@angular/core';

import { LATEST_RELEASE } from '../core/site';

type System = 'windows' | 'linux';

interface Format {
  readonly name: string;
  /** How its file ends on the release page, which lists every format of every system. */
  readonly ending: string;
  readonly who: string;
}

interface Offer {
  readonly system: System;
  readonly label: string;
  readonly action: string;
  readonly primary: Format;
  readonly others: readonly Format[];
}

/** Every link is the latest release: the site names no version and never goes stale. */
const OFFERS: readonly Offer[] = [
  {
    system: 'windows',
    label: 'Windows',
    action: "Télécharger l'installateur Windows",
    primary: {
      name: 'Installateur',
      ending: '_x64-setup.exe',
      who: "Il s'installe pour votre compte, sans droits administrateur.",
    },
    others: [
      { name: 'Paquet MSI', ending: '_x64_en-US.msi', who: 'Pour un déploiement par stratégie de groupe.' },
      {
        name: 'Exécutable portable',
        ending: '-windows.exe',
        who: "Sans installation, à lancer d'où vous voulez.",
      },
    ],
  },
  {
    system: 'linux',
    label: 'Linux',
    action: "Télécharger l'AppImage",
    primary: {
      name: 'AppImage',
      ending: '_amd64.AppImage',
      who: 'Toutes les distributions, rien à installer.',
    },
    others: [
      { name: 'Paquet .deb', ending: '_amd64.deb', who: 'Debian, Ubuntu et dérivées.' },
      { name: 'Paquet .rpm', ending: '.x86_64.rpm', who: 'Fedora, RHEL et dérivées.' },
      { name: 'Binaire seul', ending: '-linux', who: "L'exécutable, sans paquet." },
    ],
  },
];

function visitorSystem(): System {
  const agent = navigator.userAgent;
  return /linux|x11/i.test(`${navigator.platform} ${agent}`) && !/android/i.test(agent) ? 'linux' : 'windows';
}

/** The download zone: the visitor's system first, one format put forward, the others folded. */
@Component({
  selector: 'app-download-section',
  templateUrl: './download-section.html',
  styleUrl: './download-section.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class DownloadSection {
  protected readonly offers = OFFERS;
  protected readonly release = LATEST_RELEASE;
  protected readonly selected = signal<System>('windows');
  /** A phone cannot run it: say so rather than offer a file it cannot open. */
  protected readonly onPhone = signal(false);
  private readonly tabs = viewChildren<ElementRef<HTMLButtonElement>>('tab');

  constructor() {
    afterNextRender(() => {
      this.selected.set(visitorSystem());
      this.onPhone.set(/Android|iPhone|iPad|iPod/i.test(navigator.userAgent));
    });
  }

  protected select(system: System): void {
    this.selected.set(system);
  }

  /** The arrows move between the tabs, as a tab list does. */
  protected step(event: KeyboardEvent, index: number): void {
    if (event.key !== 'ArrowRight' && event.key !== 'ArrowLeft') return;
    event.preventDefault();
    const next = (index + (event.key === 'ArrowRight' ? 1 : OFFERS.length - 1)) % OFFERS.length;
    const offer = OFFERS[next];
    if (!offer) return;
    this.selected.set(offer.system);
    this.tabs()[next]?.nativeElement.focus();
  }
}
