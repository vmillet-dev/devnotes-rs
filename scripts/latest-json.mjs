#!/usr/bin/env node
// The manifest the updater reads (`latest.json`, D26), from the folder of a release's files:
// `node scripts/latest-json.mjs --dir release --base <url> --version x.y.z --notes <file>`.
//
// `cargo-packager-updater` refuses a platform without its `format`, and installs only the NSIS
// installer and the AppImage: the other signatures (.deb, .rpm) are ignored, not fatal.

import { existsSync, readFileSync, readdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { parseArgs } from 'node:util';
import { pathToFileURL } from 'node:url';

export const TARGETS = [
  { suffix: '-setup.exe', key: 'windows-x86_64', format: 'nsis' },
  { suffix: '.AppImage', key: 'linux-x86_64', format: 'appimage' },
];

/**
 * The manifest for the files of `names` (a folder's listing), whose signatures `read` returns.
 * Each `.sig` names the file it signs: an orphan, or two candidates for one platform, is an
 * error rather than a guess that might ship the file the updater cannot launch.
 */
export function manifest({ names, read, base, version, notes, now = new Date() }) {
  const platforms = {};
  const ignored = [];
  for (const sig of names.filter((name) => name.endsWith('.sig'))) {
    const asset = sig.slice(0, -'.sig'.length);
    if (!names.includes(asset)) {
      throw new Error(`orphan signature: ${sig}`);
    }
    const target = TARGETS.find(({ suffix }) => asset.endsWith(suffix));
    if (!target) {
      ignored.push(asset);
      continue;
    }
    if (platforms[target.key]) {
      throw new Error(`two artifacts for ${target.key}: ${platforms[target.key].asset} and ${asset}`);
    }
    platforms[target.key] = {
      asset,
      format: target.format,
      signature: read(sig).trim(),
      url: `${base.replace(/\/$/, '')}/${asset}`,
    };
  }
  // Windows is the priority target: a manifest without it would serve almost nobody.
  if (!platforms['windows-x86_64']) {
    throw new Error('no Windows update artifact');
  }
  if (!notes.trim()) {
    throw new Error('no release notes');
  }
  const warnings = platforms['linux-x86_64'] ? [] : ['No Linux update artifact: Windows-only manifest.'];
  for (const entry of Object.values(platforms)) {
    delete entry.asset;
  }
  // Without the "v": the updater compares the value as semver, and a prefix would make it
  // unreadable — no update would ever be offered.
  return {
    manifest: {
      version: version.replace(/^v/, ''),
      pub_date: now.toISOString(),
      notes: notes.trim(),
      platforms,
    },
    ignored,
    warnings,
  };
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  const { values } = parseArgs({
    options: {
      dir: { type: 'string' },
      base: { type: 'string' },
      version: { type: 'string' },
      notes: { type: 'string' },
    },
  });
  for (const name of ['dir', 'base', 'version', 'notes']) {
    if (!values[name]) {
      console.error(`--${name} is required`);
      process.exit(2);
    }
  }
  if (!existsSync(values.notes)) {
    console.error(`no release notes at ${values.notes}`);
    process.exit(1);
  }
  const {
    manifest: written,
    ignored,
    warnings,
  } = manifest({
    names: readdirSync(values.dir),
    read: (name) => readFileSync(join(values.dir, name), 'utf8'),
    base: values.base,
    version: values.version,
    notes: readFileSync(values.notes, 'utf8'),
  });
  if (ignored.length) {
    console.log(`Signatures ignored (formats the updater does not install): ${ignored.join(', ')}`);
  }
  for (const warning of warnings) {
    console.log(`::warning::${warning}`);
  }
  writeFileSync(join(values.dir, 'latest.json'), `${JSON.stringify(written, null, 2)}\n`);
  console.log(JSON.stringify(written, null, 2));
}
