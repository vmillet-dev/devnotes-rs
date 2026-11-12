import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { describe, it } from 'node:test';

import { checkedVersion, lockedVersion, workspaceVersion } from './version.mjs';

const TOML = `[workspace]
members = ["crates/app"]

[workspace.package]
edition = "2024"
version = "1.2.3"

[workspace.dependencies]
egui = { version = "0.37.0", default-features = false }
`;

const lock = (version) => `version = 4

[[package]]
name = "anyhow"
version = "9.9.9"

[[package]]
name = "app"
version = "${version}"
dependencies = [
 "engine",
]
`;

describe('version', () => {
  it('reads the version of [workspace.package], not of a dependency', () => {
    assert.equal(workspaceVersion(TOML), '1.2.3');
  });

  it('reads the version Cargo.lock holds for the binary', () => {
    assert.equal(lockedVersion(lock('1.2.3')), '1.2.3');
  });

  it('answers the version when both files agree', () => {
    assert.equal(checkedVersion(TOML, lock('1.2.3')), '1.2.3');
  });

  it('refuses when Cargo.lock disagrees with Cargo.toml', () => {
    assert.throws(() => checkedVersion(TOML, lock('1.2.2')), /Cargo\.lock says 1\.2\.2/);
  });

  it('refuses a Cargo.toml without a workspace version', () => {
    assert.throws(() => checkedVersion('[workspace]\n', lock('1.2.3')), /no version/);
  });

  it('agrees with the repository itself', () => {
    const version = checkedVersion(readFileSync('Cargo.toml', 'utf8'), readFileSync('Cargo.lock', 'utf8'));
    assert.match(version, /^\d+\.\d+\.\d+$/);
  });
});
