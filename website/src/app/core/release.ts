import { version } from '../../../../package.json';
import { Lang } from './lang';
import RELEASE from './release-files.json';
import { REPOSITORY } from './site';

/**
 * The version the site offers: the application's own, read at build time. A bump changes it,
 * so the site is rebuilt and published once that version's release exists (docs/releasing.md);
 * `website/scripts/fetch-release.mjs` reads its files' sizes and SHA-256 before the build, and
 * `finish-build.mjs` refuses a build whose links lead nowhere.
 */
export const VERSION = version;

const FILES = `${REPOSITORY}/releases/download/v${version}`;

export type System = 'windows' | 'linux';

export interface ReleaseFile {
  readonly name: string;
  readonly file: string;
  readonly url: string;
  readonly who: string;
  readonly size: string;
  readonly sha256: string;
}

export interface Offer {
  readonly system: System;
  readonly label: string;
  readonly action: string;
  readonly primary: ReleaseFile;
  readonly others: readonly ReleaseFile[];
}

const KNOWN = RELEASE.files as Readonly<Record<string, { readonly size: number; readonly sha256: string }>>;

function megabytes(bytes: number, lang: Lang): string {
  const amount = new Intl.NumberFormat(lang === 'fr' ? 'fr-FR' : 'en-GB', {
    maximumFractionDigits: 1,
  }).format(bytes / 1_000_000);
  return lang === 'fr' ? `${amount} Mo` : `${amount} MB`;
}

const file = (lang: Lang, name: string, fileName: string, who: string): ReleaseFile => ({
  name,
  file: fileName,
  url: `${FILES}/${fileName}`,
  who,
  size: KNOWN[fileName] ? megabytes(KNOWN[fileName].size, lang) : '',
  sha256: KNOWN[fileName]?.sha256 ?? '',
});

/** One format put forward per system, the others offered beside it. Named as `release.yml` writes them. */
export function offers(lang: Lang): readonly Offer[] {
  const fr = lang === 'fr';
  return [
    {
      system: 'windows',
      label: 'Windows',
      action: fr ? "Télécharger l'installateur" : 'Download the installer',
      primary: file(
        lang,
        fr ? 'Installateur' : 'Installer',
        `devnotes_${version}_x64-setup.exe`,
        fr
          ? "Il s'installe pour votre compte, sans droits administrateur."
          : 'Installs for your account, no administrator rights needed.',
      ),
      others: [
        file(
          lang,
          fr ? 'Paquet MSI' : 'MSI package',
          `devnotes_${version}_x64_en-US.msi`,
          fr ? 'Pour un déploiement par stratégie de groupe.' : 'For deployment through Group Policy.',
        ),
        file(
          lang,
          fr ? 'Exécutable portable' : 'Portable executable',
          `devnotes-${version}-windows.exe`,
          fr ? "Sans installation, à lancer d'où vous voulez." : 'Nothing to install, run it from anywhere.',
        ),
      ],
    },
    {
      system: 'linux',
      label: 'Linux',
      action: fr ? "Télécharger l'AppImage" : 'Download the AppImage',
      primary: file(
        lang,
        'AppImage',
        `devnotes_${version}_amd64.AppImage`,
        fr ? 'Toutes les distributions, rien à installer.' : 'Any distribution, nothing to install.',
      ),
      others: [
        file(
          lang,
          fr ? 'Paquet .deb' : '.deb package',
          `devnotes_${version}_amd64.deb`,
          fr ? 'Debian, Ubuntu et dérivées.' : 'Debian, Ubuntu and derivatives.',
        ),
        file(
          lang,
          fr ? 'Paquet .rpm' : '.rpm package',
          `devnotes-${version}-1.x86_64.rpm`,
          fr ? 'Fedora, RHEL et dérivées.' : 'Fedora, RHEL and derivatives.',
        ),
        file(
          lang,
          fr ? 'Binaire seul' : 'Bare binary',
          `devnotes-${version}-linux`,
          fr ? "L'exécutable, sans paquet." : 'The executable, without a package.',
        ),
      ],
    },
  ];
}

/** The visitor's system, from what the browser says of itself; Windows when it says nothing. */
export function visitorSystem(): System {
  const agent = navigator.userAgent;
  return /linux|x11/i.test(`${navigator.platform} ${agent}`) && !/android/i.test(agent) ? 'linux' : 'windows';
}

/** The checksums of every file of the release, to check a download against. */
export const CHECKSUMS = `${FILES}/SHA256SUMS.txt`;

/** This version's page on GitHub, with its notes and every file. */
export const RELEASE_PAGE = `${REPOSITORY}/releases/tag/v${version}`;
