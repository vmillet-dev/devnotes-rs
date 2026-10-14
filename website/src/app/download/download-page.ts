import { ChangeDetectionStrategy, Component, afterNextRender, computed, inject, signal } from '@angular/core';
import { RouterLink } from '@angular/router';

import { Lang, downloadPath } from '../core/lang';
import { Language } from '../core/language';
import { CHECKSUMS, RELEASE_PAGE, System, VERSION, offers, visitorSystem } from '../core/release';
import { Seo } from '../core/seo';
import { CHANGELOG } from '../core/site';
import { DocBlocks } from '../docs/doc-blocks';
import { docPath } from '../docs/doc-index';
import { snippet } from '../docs/doc.model';

const LINUX_COMMANDS = `chmod +x devnotes_${VERSION}_amd64.AppImage && ./devnotes_${VERSION}_amd64.AppImage
sudo apt install ./devnotes_${VERSION}_amd64.deb      # Debian, Ubuntu
sudo dnf install ./devnotes-${VERSION}-1.x86_64.rpm   # Fedora, RHEL`;

const CHECK_COMMANDS = `sha256sum -c SHA256SUMS.txt --ignore-missing     # Linux
Get-FileHash .\\devnotes_*_x64-setup.exe         # Windows, PowerShell`;

const FR = {
  title: `Télécharger DevNotes ${VERSION} pour Windows et Linux`,
  description: `DevNotes ${VERSION} pour Windows et Linux : l'installateur, l'AppImage, les paquets .deb, .rpm et MSI, et les exécutables portables. Gratuit et libre.`,
  label: 'Télécharger',
  heading: 'Télécharger DevNotes',
  lede: `La version ${VERSION}, pour Windows et Linux. Gratuite et libre, sans compte : tout reste sur votre machine.`,
  phone: "DevNotes s'installe sur un ordinateur. Ouvrez cette page depuis votre poste pour le télécharger.",
  systems: 'Les fichiers, par système',
  yours: 'Votre système',
  arch: '64 bits',
  unsigned:
    'Non signé : au premier lancement, SmartScreen demande une confirmation. Choisissez Informations complémentaires, puis Exécuter quand même.',
  executable: 'Rendez-la exécutable avec chmod +x, puis lancez-la.',
  others: 'Autres formats',
  notes: `Notes de la version ${VERSION}`,
  mac: "Pas de version macOS pour l'instant : elle ne peut pas être testée ici, et Gatekeeper réclame un compte Apple Developer payant.",
  install: 'Installer',
  onWindows: 'Sous Windows',
  windows:
    "Lancez l'installateur : il s'installe pour votre compte, sans droits d'administrateur. L'avertissement de SmartScreen porte sur l'éditeur, DevNotes n'ayant pas de certificat de signature de code, pas sur le fichier : son empreinte est plus bas.",
  onLinux: 'Sous Linux',
  linux:
    "Une AppImage se rend exécutable puis se lance ; un paquet s'installe avec le gestionnaire de la distribution.",
  linuxTitle: 'Installer sous Linux',
  verify: 'Vérifier un fichier',
  verifyLede: "L'empreinte SHA-256 de chaque fichier, telle que la version la publie dans",
  file: 'Fichier',
  size: 'Taille',
  checkTitle: 'Vérifier un fichier téléchargé',
  after: 'Et ensuite',
  updates: 'Les mises à jour',
  updatesText:
    "DevNotes vérifie lui-même s'il existe une version plus récente et la propose, avec ses notes. Elles sont signées et vérifiées avant d'être installées.",
  changed: 'Ce qui a changé',
  changedBefore: 'Le',
  changelog: 'journal des modifications',
  changedMiddle: 'complet. Pour bien démarrer, la',
  start: 'prise en main',
};

const EN: typeof FR = {
  title: `Download DevNotes ${VERSION} for Windows and Linux`,
  description: `DevNotes ${VERSION} for Windows and Linux: the installer, the AppImage, the .deb, .rpm and MSI packages, and the portable executables. Free and open source.`,
  label: 'Download',
  heading: 'Download DevNotes',
  lede: `Version ${VERSION}, for Windows and Linux. Free and open source, no account: everything stays on your machine.`,
  phone: 'DevNotes installs on a computer. Open this page from your desktop to download it.',
  systems: 'The files, by system',
  yours: 'Your system',
  arch: '64-bit',
  unsigned:
    'Unsigned: on first launch, SmartScreen asks for confirmation. Choose More info, then Run anyway.',
  executable: 'Make it executable with chmod +x, then run it.',
  others: 'Other formats',
  notes: `Release notes for ${VERSION}`,
  mac: 'No macOS version for now: it cannot be tested here, and Gatekeeper requires a paid Apple Developer account.',
  install: 'Installing',
  onWindows: 'On Windows',
  windows:
    "Run the installer: it installs for your account, without administrator rights. SmartScreen's warning is about the publisher, DevNotes having no code signing certificate, not about the file: its fingerprint is below.",
  onLinux: 'On Linux',
  linux:
    "An AppImage is made executable then run; a package installs with the distribution's package manager.",
  linuxTitle: 'Installing on Linux',
  verify: 'Checking a file',
  verifyLede: 'The SHA-256 fingerprint of each file, as the release publishes it in',
  file: 'File',
  size: 'Size',
  checkTitle: 'Checking a downloaded file',
  after: 'And then',
  updates: 'Updates',
  updatesText:
    'DevNotes checks by itself whether a newer version exists and offers it, with its notes. Updates are signed and verified before they are installed.',
  changed: 'What changed',
  changedBefore: 'The',
  changelog: 'full changelog',
  changedMiddle: 'is on GitHub. To get going, see',
  start: 'Getting started',
};

const TEXT: Readonly<Record<Lang, typeof FR>> = { fr: FR, en: EN };

@Component({
  selector: 'app-download-page',
  imports: [RouterLink, DocBlocks],
  templateUrl: './download-page.html',
  styleUrl: './download-page.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class DownloadPage {
  private readonly lang = inject(Language).lang;
  protected readonly text = computed(() => TEXT[this.lang()]);
  protected readonly offers = computed(() => offers(this.lang()));
  /** Every file of the release, to check against its fingerprint. */
  protected readonly files = computed(() =>
    this.offers().flatMap((offer) => [offer.primary, ...offer.others]),
  );
  protected readonly startPath = computed(() => docPath('start', this.lang()));
  protected readonly linuxInstall = computed(() => [snippet(this.text().linuxTitle, 'sh', LINUX_COMMANDS)]);
  protected readonly check = computed(() => [snippet(this.text().checkTitle, 'sh', CHECK_COMMANDS)]);
  protected readonly version = VERSION;
  protected readonly checksums = CHECKSUMS;
  protected readonly releasePage = RELEASE_PAGE;
  protected readonly changelog = CHANGELOG;
  /** Put first and marked once the browser says which it is; Windows until then. */
  protected readonly yours = signal<System | null>(null);
  /** A phone cannot run it: say so rather than offer a file it cannot open. */
  protected readonly onPhone = signal(false);

  constructor() {
    const lang = this.lang();
    const text = TEXT[lang];
    inject(Seo).describe({
      title: text.title,
      description: text.description,
      path: downloadPath(lang),
      lang,
    });
    afterNextRender(() => {
      this.yours.set(visitorSystem());
      this.onPhone.set(/Android|iPhone|iPad|iPod/i.test(navigator.userAgent));
    });
  }
}
