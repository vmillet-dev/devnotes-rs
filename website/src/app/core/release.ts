import { version } from '../../../../package.json';
import { REPOSITORY } from './site';

/**
 * The version the site offers: the application's own, read at build time. A bump changes it,
 * so the site is rebuilt and published once that version's release exists (docs/releasing.md);
 * `website/scripts/finish-build.mjs` refuses a build whose links lead nowhere.
 */
export const VERSION = version;

const FILES = `${REPOSITORY}/releases/download/v${version}`;

export type System = 'windows' | 'linux';

export interface ReleaseFile {
  readonly name: string;
  readonly file: string;
  readonly url: string;
  readonly who: string;
}

const file = (name: string, fileName: string, who: string): ReleaseFile => ({
  name,
  file: fileName,
  url: `${FILES}/${fileName}`,
  who,
});

export interface Offer {
  readonly system: System;
  readonly label: string;
  readonly primary: ReleaseFile;
  readonly others: readonly ReleaseFile[];
}

/** One format put forward per system, the others offered beside it. Named as `release.yml` writes them. */
export const OFFERS: readonly Offer[] = [
  {
    system: 'windows',
    label: 'Windows',
    primary: file(
      'Installateur',
      `devnotes_${version}_x64-setup.exe`,
      "Il s'installe pour votre compte, sans droits administrateur.",
    ),
    others: [
      file('Paquet MSI', `devnotes_${version}_x64_en-US.msi`, 'Pour un déploiement par stratégie de groupe.'),
      file(
        'Exécutable portable',
        `devnotes-${version}-windows.exe`,
        "Sans installation, à lancer d'où vous voulez.",
      ),
    ],
  },
  {
    system: 'linux',
    label: 'Linux',
    primary: file(
      'AppImage',
      `devnotes_${version}_amd64.AppImage`,
      'Toutes les distributions, rien à installer.',
    ),
    others: [
      file('Paquet .deb', `devnotes_${version}_amd64.deb`, 'Debian, Ubuntu et dérivées.'),
      file('Paquet .rpm', `devnotes-${version}-1.x86_64.rpm`, 'Fedora, RHEL et dérivées.'),
      file('Binaire seul', `devnotes-${version}-linux`, "L'exécutable, sans paquet."),
    ],
  },
];

/** The checksums of every file of the release, to check a download against. */
export const CHECKSUMS = `${FILES}/SHA256SUMS.txt`;

/** This version's page on GitHub, with its notes and every file. */
export const RELEASE_PAGE = `${REPOSITORY}/releases/tag/v${version}`;
