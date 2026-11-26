import assert from 'node:assert/strict';
import { describe, it } from 'node:test';

import { preview, renamed } from './release-preview.mjs';

const RELEASE = {
  repo: 'owner/name',
  previous: '1.1.0',
  version: '1.2.0',
  commit: '3dc69d1d',
  subject: 'Website: language button named in full (#1652)',
  diff: '+## [1.2.0] - 2026-12-01\n+\n+- Tabs: raised (#1)',
  prompt: '### ✨ Added\n\n- Tabs: raised (#1)\n',
  files: ['devnotes-1.2.0-linux', 'latest.json'],
};

describe('renamed', () => {
  it('puts the new version in the names of the last release', () => {
    const files = ['devnotes_1.1.0_x64-setup.exe', 'devnotes-1.1.0-linux', 'latest.json'];

    assert.deepEqual(renamed(files, '1.1.0', '1.2.0'), [
      'devnotes_1.2.0_x64-setup.exe',
      'devnotes-1.2.0-linux',
      'latest.json',
    ]);
  });
});

describe('preview', () => {
  it('says what would be committed, tagged and published, and from what', () => {
    const text = preview(RELEASE);

    for (const said of [
      '1.1.0 → **1.2.0**',
      '`3dc69d1d` Website: language button named in full (#1652)',
      '`chore(release): v1.2.0`',
      '`v1.2.0`, sur ce commit',
      'https://github.com/owner/name/compare/v1.1.0...v1.2.0',
      '`devnotes-1.2.0-linux`<br>`latest.json`',
    ]) {
      assert.ok(text.includes(said), said);
    }
  });

  it('shows the changelog as a diff and the prompt as it is written', () => {
    const text = preview(RELEASE);

    assert.ok(text.includes(`\`\`\`diff\n${RELEASE.diff}\n\`\`\``));
    assert.ok(text.includes(`\`\`\`markdown\n${RELEASE.prompt.trimEnd()}\n\`\`\``));
  });

  it('is not closed early by a fence in the diff, nor split by a bar in a title', () => {
    const text = preview({ ...RELEASE, diff: '+```js\n+x\n+```', subject: 'Tables: a | b' });

    assert.ok(text.includes('````diff\n+```js\n+x\n+```\n````'));
    assert.ok(text.includes('Tables: a \\| b |'));
  });

  it('does without a release before it', () => {
    const text = preview({ ...RELEASE, previous: '', files: [] });

    assert.ok(text.includes('**1.2.0**, la première'));
    assert.ok(!text.includes('/compare/'));
  });
});
