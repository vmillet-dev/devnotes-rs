// Before `ng build website`: the size and SHA-256 of every file of the version in
// package.json, read from its GitHub release, for the download page to show. A release not
// published yet stops the build here, as its download links would lead nowhere.
import { readFileSync, writeFileSync } from 'node:fs';

const REPOSITORY = 'vmillet-dev/devnotes-rs';
const OUT = 'website/src/app/core/release-files.json';

const { version } = JSON.parse(readFileSync('package.json', 'utf8'));
const answer = await fetch(`https://api.github.com/repos/${REPOSITORY}/releases/tags/v${version}`, {
  headers: { Accept: 'application/vnd.github+json' },
});
if (!answer.ok) {
  console.error(
    `No published release v${version} on GitHub (${answer.status}): the download links would lead nowhere.`,
  );
  process.exit(1);
}
const release = await answer.json();

const sums = release.assets.find((asset) => asset.name === 'SHA256SUMS.txt');
if (!sums) {
  console.error(`The release v${version} has no SHA256SUMS.txt.`);
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
