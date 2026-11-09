import assert from 'node:assert/strict';
import { describe, it } from 'node:test';

import { manifest } from './latest-json.mjs';

const SIGNATURES = {
  'devnotes_1.0.1_x64-setup.exe.sig': 'sig-nsis\n',
  'devnotes_1.0.1_x86_64.AppImage.sig': 'sig-appimage\n',
  'devnotes_1.0.1_amd64.deb.sig': 'sig-deb\n',
};
const RELEASE = [
  'devnotes_1.0.1_x64-setup.exe',
  'devnotes_1.0.1_x86_64.AppImage',
  'devnotes_1.0.1_amd64.deb',
  'devnotes-1.0.1-windows.exe',
  ...Object.keys(SIGNATURES),
];

const build = (names, overrides = {}) =>
  manifest({
    names,
    read: (name) => SIGNATURES[name],
    base: 'https://example.org/download/v1.0.1/',
    version: 'v1.0.1',
    notes: '## What changed\n',
    now: new Date('2026-11-09T12:00:00Z'),
    ...overrides,
  });

describe('latest.json', () => {
  it('lists the NSIS installer and the AppImage with their format, signature and address', () => {
    const { manifest: written, ignored } = build(RELEASE);

    assert.deepEqual(written, {
      version: '1.0.1',
      pub_date: '2026-11-09T12:00:00.000Z',
      notes: '## What changed',
      platforms: {
        'windows-x86_64': {
          format: 'nsis',
          signature: 'sig-nsis',
          url: 'https://example.org/download/v1.0.1/devnotes_1.0.1_x64-setup.exe',
        },
        'linux-x86_64': {
          format: 'appimage',
          signature: 'sig-appimage',
          url: 'https://example.org/download/v1.0.1/devnotes_1.0.1_x86_64.AppImage',
        },
      },
    });
    assert.deepEqual(ignored, ['devnotes_1.0.1_amd64.deb']);
  });

  it('never takes the standalone executable for the installer', () => {
    const { manifest: written } = build(RELEASE);

    assert.doesNotMatch(written.platforms['windows-x86_64'].url, /windows\.exe$/);
  });

  it('refuses a signature without its file', () => {
    assert.throws(() => build(RELEASE.filter((name) => name !== 'devnotes_1.0.1_amd64.deb')), /orphan/);
  });

  it('refuses a release without its Windows installer', () => {
    const names = RELEASE.filter((name) => !name.includes('setup'));

    assert.throws(() => build(names), /no Windows/);
  });

  it('warns when Linux is missing but still writes the manifest', () => {
    const names = RELEASE.filter((name) => !name.includes('AppImage'));

    const { manifest: written, warnings } = build(names);

    assert.equal(warnings.length, 1);
    assert.deepEqual(Object.keys(written.platforms), ['windows-x86_64']);
  });

  it('refuses empty notes', () => {
    assert.throws(() => build(RELEASE, { notes: '  \n' }), /no release notes/);
  });
});
