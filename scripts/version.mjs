#!/usr/bin/env node
// The one version of the workspace (D27): `[workspace.package] version` in the root
// Cargo.toml, which Cargo.lock repeats for every crate. `release.yml` reads it here and refuses
// a release when the two files disagree: it builds with `--locked`.

import { readFileSync } from 'node:fs';
import { pathToFileURL } from 'node:url';

/** The crate whose entry in Cargo.lock is compared: the binary's. */
const CRATE = 'app';

/** `version` inside `[workspace.package]`, or `null`. */
export function workspaceVersion(cargoToml) {
  let inside = false;
  for (const line of cargoToml.split('\n')) {
    const header = line.match(/^\s*\[([^\]]+)\]\s*$/);
    if (header) {
      inside = header[1].trim() === 'workspace.package';
      continue;
    }
    const version = inside && line.match(/^\s*version\s*=\s*"([^"]+)"/);
    if (version) {
      return version[1];
    }
  }
  return null;
}

/** The version Cargo.lock holds for `name`, or `null`. */
export function lockedVersion(cargoLock, name = CRATE) {
  for (const entry of cargoLock.split('[[package]]')) {
    if (new RegExp(`^name = "${name}"$`, 'm').test(entry)) {
      return entry.match(/^version = "([^"]+)"$/m)?.[1] ?? null;
    }
  }
  return null;
}

/** The version both files agree on; throws when either has none or they differ. */
export function checkedVersion(cargoToml, cargoLock) {
  const declared = workspaceVersion(cargoToml);
  if (!declared) {
    throw new Error('no version in [workspace.package] of Cargo.toml');
  }
  const locked = lockedVersion(cargoLock);
  if (locked !== declared) {
    throw new Error(`Cargo.lock says ${locked ?? 'nothing'} for ${CRATE}, Cargo.toml says ${declared}`);
  }
  return declared;
}

if (import.meta.url === pathToFileURL(process.argv[1]).href) {
  try {
    console.log(checkedVersion(readFileSync('Cargo.toml', 'utf8'), readFileSync('Cargo.lock', 'utf8')));
  } catch (error) {
    console.error(`::error::${error.message}`);
    process.exit(1);
  }
}
