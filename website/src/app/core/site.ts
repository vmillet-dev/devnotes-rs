/** The repository the application is built from: the releases, the source, the changelog. */
export const REPOSITORY = 'https://github.com/vmillet-dev/devnotes-rs';

/** Always the newest release: the site names no version and never goes stale. */
export const LATEST_RELEASE = `${REPOSITORY}/releases/latest`;

export const CHANGELOG = `${REPOSITORY}/blob/main/CHANGELOG.md`;

export const LICENSE = `${REPOSITORY}/blob/main/LICENSE`;

/** An absolute address on this site, for the canonical link and the social cards. */
export function absolute(path: string): string {
  return new URL(path, SITE_URL).toString();
}
