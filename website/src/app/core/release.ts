import { Lang } from './lang';
import RELEASE from './release-files.json';
import { REPOSITORY } from './site';

/**
 * The version the site offers: the latest published release. `scripts/fetch-release.mjs` reads
 * it, with its files' sizes and SHA-256, before every build (`website.yml` builds again after each
 * release), and `finish-build.mjs` refuses a build whose links lead nowhere.
 */
export const VERSION = RELEASE.version;

const FILES = `${REPOSITORY}/releases/download/v${VERSION}`;

export type System = 'windows' | 'macos' | 'linux';

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

const APPLE_SILICON = `devnotes_${VERSION}_aarch64.dmg`;
const INTEL = `devnotes_${VERSION}_x64.dmg`;

/** The first releases had no macOS files, and a release whose macOS build failed has none either. */
const HAS_MACOS = APPLE_SILICON in KNOWN && INTEL in KNOWN;

/** The systems the release is for, as a sentence names them. */
export function systems(lang: Lang): string {
  const and = lang === 'fr' ? 'et' : 'and';
  return HAS_MACOS ? `Windows, macOS ${and} Linux` : `Windows ${and} Linux`;
}

/** One format put forward per system, the others offered beside it. Named as `release.yml` writes them. */
export function offers(lang: Lang): readonly Offer[] {
  const fr = lang === 'fr';
  const macos: Offer = {
    system: 'macos',
    label: 'macOS',
    action: fr ? 'Télécharger pour Apple Silicon' : 'Download for Apple Silicon',
    primary: file(
      lang,
      'Apple Silicon',
      APPLE_SILICON,
      fr ? 'Les Mac à puce Apple (M1 et suivantes).' : 'Macs with an Apple chip (M1 and later).',
    ),
    others: [
      file(lang, 'Intel', INTEL, fr ? 'Les Mac à processeur Intel.' : 'Macs with an Intel processor.'),
    ],
  };
  return [
    {
      system: 'windows',
      label: 'Windows',
      action: fr ? "Télécharger l'installateur" : 'Download the installer',
      primary: file(
        lang,
        fr ? 'Installateur' : 'Installer',
        `devnotes_${VERSION}_x64-setup.exe`,
        fr
          ? "Il s'installe pour votre compte, sans droits administrateur."
          : 'Installs for your account, no administrator rights needed.',
      ),
      others: [
        file(
          lang,
          fr ? 'Exécutable portable' : 'Portable executable',
          `devnotes-${VERSION}-windows.exe`,
          fr ? "Sans installation, à lancer d'où vous voulez." : 'Nothing to install, run it from anywhere.',
        ),
      ],
    },
    ...(HAS_MACOS ? [macos] : []),
    {
      system: 'linux',
      label: 'Linux',
      action: fr ? "Télécharger l'AppImage" : 'Download the AppImage',
      primary: file(
        lang,
        'AppImage',
        `devnotes_${VERSION}_x86_64.AppImage`,
        fr ? 'Toutes les distributions, rien à installer.' : 'Any distribution, nothing to install.',
      ),
      others: [
        file(
          lang,
          fr ? 'Paquet .deb' : '.deb package',
          `devnotes_${VERSION}_amd64.deb`,
          fr ? 'Debian, Ubuntu et dérivées.' : 'Debian, Ubuntu and derivatives.',
        ),
        file(
          lang,
          fr ? 'Paquet .rpm' : '.rpm package',
          `devnotes-${VERSION}.x86_64.rpm`,
          fr ? 'Fedora, RHEL et dérivées.' : 'Fedora, RHEL and derivatives.',
        ),
        file(
          lang,
          fr ? 'Binaire seul' : 'Bare binary',
          `devnotes-${VERSION}-linux`,
          fr ? "L'exécutable, sans paquet." : 'The executable, without a package.',
        ),
      ],
    },
  ];
}

/** The visitor's system, from what the browser says of itself; Windows when it says nothing. */
export function visitorSystem(): System {
  const agent = navigator.userAgent;
  const said = `${navigator.platform} ${agent}`;
  if (HAS_MACOS && /mac/i.test(said) && !/iphone|ipad|ipod/i.test(agent)) {
    return 'macos';
  }
  return /linux|x11/i.test(said) && !/android/i.test(agent) ? 'linux' : 'windows';
}

/** The checksums of every file of the release, to check a download against. */
export const CHECKSUMS = `${FILES}/SHA256SUMS.txt`;

/** This version's page on GitHub, with its notes and every file. */
export const RELEASE_PAGE = `${REPOSITORY}/releases/tag/v${VERSION}`;
