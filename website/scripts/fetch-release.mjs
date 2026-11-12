// Before `ng build`: the latest published release of the application, its version and the
// size and SHA-256 of each file, for the download page to name and to show. The site follows
// the releases, never the source tree: it offers what can be downloaded now.
import { writeFileSync } from 'node:fs';

const REPOSITORY = 'vmillet-dev/devnotes-rs';
const OUT = 'src/app/core/release-files.json';

const headers = { Accept: 'application/vnd.github+json' };
if (process.env.GITHUB_TOKEN) headers.Authorization = `Bearer ${process.env.GITHUB_TOKEN}`;

const answer = await fetch(`https://api.github.com/repos/${REPOSITORY}/releases/latest`, { headers });
if (!answer.ok) {
  console.error(`No published release on GitHub (${answer.status}): the download links would lead nowhere.`);
  process.exit(1);
}
const release = await answer.json();
const version = release.tag_name.replace(/^v/, '');

const sums = release.assets.find((asset) => asset.name === 'SHA256SUMS.txt');
if (!sums) {
  console.error(`The release ${release.tag_name} has no SHA256SUMS.txt.`);
  process.exit(1);
}
const listed = new Map(
  (await (await fetch(sums.browser_download_url)).text())
    .split('\n')
    .map((line) => line.trim().split(/\s+\*?/))
    .filter(([hash, name]) => hash && name)
    .map(([hash, name]) => [name, hash]),
);

const files = Object.fromEntries(
  release.assets
    .filter((asset) => listed.has(asset.name) && !asset.name.endsWith('.sig') && asset.name !== 'latest.json')
    .map((asset) => [asset.name, { size: asset.size, sha256: listed.get(asset.name) }]),
);
writeFileSync(OUT, `${JSON.stringify({ version, files }, null, 2)}\n`);
console.log(`v${version}: ${Object.keys(files).length} files, their sizes and SHA-256 written to ${OUT}.`);
