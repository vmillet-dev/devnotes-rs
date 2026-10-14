import { ChangeDetectionStrategy, Component, afterNextRender, inject, signal } from '@angular/core';
import { RouterLink } from '@angular/router';

import { CHECKSUMS, OFFERS, RELEASE_PAGE, System, VERSION } from '../core/release';
import { Seo } from '../core/seo';
import { CHANGELOG } from '../core/site';
import { DocBlocks } from '../docs/doc-blocks';
import { snippet } from '../docs/doc.model';

const LINUX_INSTALL = snippet(
  'Installer sous Linux',
  'sh',
  `chmod +x devnotes_${VERSION}_amd64.AppImage && ./devnotes_${VERSION}_amd64.AppImage\nsudo apt install ./devnotes_${VERSION}_amd64.deb      # Debian, Ubuntu\nsudo dnf install ./devnotes-${VERSION}-1.x86_64.rpm   # Fedora, RHEL`,
);

const CHECK = snippet(
  'Vérifier un fichier téléchargé',
  'sh',
  'sha256sum -c SHA256SUMS.txt --ignore-missing     # Linux\nGet-FileHash .\\devnotes_*_x64-setup.exe          # Windows, PowerShell',
);

function visitorSystem(): System {
  const agent = navigator.userAgent;
  return /linux|x11/i.test(`${navigator.platform} ${agent}`) && !/android/i.test(agent) ? 'linux' : 'windows';
}

@Component({
  selector: 'app-download-page',
  imports: [RouterLink, DocBlocks],
  templateUrl: './download-page.html',
  styleUrl: './download-page.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class DownloadPage {
  protected readonly version = VERSION;
  protected readonly offers = OFFERS;
  protected readonly checksums = CHECKSUMS;
  protected readonly releasePage = RELEASE_PAGE;
  protected readonly changelog = CHANGELOG;
  protected readonly linuxInstall = [LINUX_INSTALL];
  protected readonly check = [CHECK];
  /** Put first and marked once the browser says which it is; Windows until then. */
  protected readonly yours = signal<System | null>(null);
  /** A phone cannot run it: say so rather than offer a file it cannot open. */
  protected readonly onPhone = signal(false);

  constructor() {
    inject(Seo).describe({
      title: `Télécharger DevNotes ${VERSION} pour Windows et Linux`,
      description: `DevNotes ${VERSION} pour Windows et Linux : l'installateur, l'AppImage, les paquets .deb, .rpm et MSI, et les exécutables portables. Gratuit et libre.`,
      path: '/telecharger',
    });
    afterNextRender(() => {
      this.yours.set(visitorSystem());
      this.onPhone.set(/Android|iPhone|iPad|iPod/i.test(navigator.userAgent));
    });
  }
}
