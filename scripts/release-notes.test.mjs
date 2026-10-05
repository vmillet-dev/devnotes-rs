import assert from 'node:assert/strict';
import { describe, it } from 'node:test';

import {
  assertRenderable,
  cleanTitle,
  extractSection,
  groupEntries,
  isReleaseCommit,
  kindOf,
  parsePrNumber,
  renderSections,
  sectionFor,
  splice,
} from './release-notes.mjs';

describe('cleanTitle', () => {
  it('drops the kind prefix and the pull request number', () => {
    assert.equal(cleanTitle('(refactor) Drop the DTO aliases (#56)'), 'Drop the DTO aliases');
    assert.equal(cleanTitle('(Refacto) Enhance code safety (#8)'), 'Enhance code safety');
    assert.equal(cleanTitle('fix(ui): Move the titlebar menus'), 'Move the titlebar menus');
  });

  it('cuts a `summary: details` title at the colon', () => {
    assert.equal(
      cleanTitle('(feat) Add e2e testing harness: configure Tauri build, integrate WebDriverIO'),
      'Add e2e testing harness',
    );
  });

  it('keeps a head too short to be a phrase of its own', () => {
    assert.equal(cleanTitle('Note: the panel is read-only'), 'Note: the panel is read-only');
  });

  it('leaves prose alone and strips a leading list marker', () => {
    assert.equal(cleanTitle('Move the titlebar menus to the left'), 'Move the titlebar menus to the left');
    assert.equal(cleanTitle('- Move the menus'), 'Move the menus');
  });
});

describe('kindOf', () => {
  it('reads the spellings this history uses', () => {
    assert.equal(kindOf('(feat) X'), 'feat');
    assert.equal(kindOf('(Refacto) X'), 'refactor');
    assert.equal(kindOf('(TEST) X'), 'test');
    assert.equal(kindOf('fix(ui): X'), 'fix');
  });

  it('refuses a word it does not know', () => {
    assert.equal(kindOf('Note: the panel is read-only'), null);
    assert.equal(kindOf('Merge pull request #66 from x'), null);
  });
});

describe('parsePrNumber', () => {
  it('only reads a number anchored at the end', () => {
    assert.equal(parsePrNumber('Drop the DTO aliases (#56)'), 56);
    assert.equal(parsePrNumber('Fix (#1) then (#2)'), 2);
    assert.equal(parsePrNumber('Drop (#56) and more'), null);
    assert.equal(parsePrNumber('No number here'), null);
  });
});

describe('isReleaseCommit', () => {
  it('catches the release and bump commits', () => {
    assert.ok(isReleaseCommit('chore(release): v0.1.3'));
    assert.ok(isReleaseCommit('Bump version to 0.1.2'));
  });

  it('does not catch a dependency bump', () => {
    assert.equal(isReleaseCommit('Bump the Angular dependency'), false);
  });
});

describe('sectionFor', () => {
  it('lets the pull request label win over the ticket', () => {
    // The real #52: labelled `bug`, closing issue #42 labelled `enhancement`.
    const { section, via } = sectionFor({
      labels: ['bug'],
      issueLabels: [{ issue: 42, label: 'enhancement' }],
      title: '(fix) Move the titlebar menus to the left',
    });
    assert.equal(section, 'fixed');
    assert.match(via, /label PR/);
  });

  it('lets the ticket label win over the title prefix', () => {
    // The real #58: no label, closing issue #44 labelled `refactor`, title `(feat)`.
    const { section, via } = sectionFor({
      issueLabels: [{ issue: 44, label: 'refactor' }],
      title: '(feat) Describe the application from Cargo.toml',
    });
    assert.equal(section, 'internal');
    assert.equal(via, 'label ticket #44 (refactor)');
  });

  it('breaks a tie on the order of the table', () => {
    assert.equal(sectionFor({ labels: ['bug', 'enhancement'] }).section, 'added');
  });

  it('steps over an unmapped label rather than stopping on it', () => {
    const { section, via } = sectionFor({
      issueLabels: [{ issue: 40, label: 'question' }],
      title: '(fix) Something',
    });
    assert.equal(section, 'fixed');
    assert.equal(via, 'préfixe (fix)');
  });

  it('files what nothing classifies under the hood', () => {
    assert.deepEqual(sectionFor({ title: 'Something nobody labelled' }), {
      section: 'internal',
      via: 'défaut',
    });
  });
});

describe('the dependencies section', () => {
  it('takes a pull request labelled dependencies, as Dependabot labels its own', () => {
    assert.equal(
      sectionFor({ labels: ['dependencies'], title: 'Bump tauri from 2.11.5 to 2.11.6' }).section,
      'dependencies',
    );
    assert.equal(sectionFor({ labels: ['dependencies', 'rust'] }).section, 'dependencies');
  });

  it('takes nothing else, not even a deps prefix', () => {
    assert.equal(sectionFor({ title: 'deps: refresh the lockfile' }).section, 'internal');
    assert.equal(sectionFor({ labels: ['refactor'] }).section, 'internal');
  });

  it('gives way to any other mapped label the pull request wears', () => {
    assert.equal(sectionFor({ labels: ['dependencies', 'enhancement'] }).section, 'added');
    assert.equal(sectionFor({ labels: ['dependencies', 'refactor'] }).section, 'internal');
  });

  it('comes last, after Under the hood, and is not written when empty', () => {
    const groups = groupEntries([
      { section: 'dependencies', text: 'Bump jsdom from 29.1.1 to 30.1.1 (#562)' },
      { section: 'internal', text: 'Drop the DTO aliases (#56)' },
      { section: 'added', text: 'A sample for every tool (#600)' },
    ]);
    assert.deepEqual(
      groups.map((group) => group.heading),
      ['✨ Added', '🧰 Under the hood', '📦 Dependencies'],
    );

    const without = groupEntries([{ section: 'internal', text: 'Drop the DTO aliases (#56)' }]);
    assert.deepEqual(
      without.map((group) => group.heading),
      ['🧰 Under the hood'],
    );
  });
});

describe('groupEntries', () => {
  it('respects the table order, drops empty sections and keeps entry order', () => {
    const groups = groupEntries([
      { section: 'internal', text: 'b' },
      { section: 'added', text: 'a1' },
      { section: 'internal', text: 'c' },
      { section: 'added', text: 'a2' },
    ]);

    assert.deepEqual(
      groups.map((group) => group.heading),
      ['✨ Added', '🧰 Under the hood'],
    );
    assert.deepEqual(groups[0].items, ['a1', 'a2']);
    assert.deepEqual(groups[1].items, ['b', 'c']);
  });
});

describe('renderSections', () => {
  it('emits the exact bytes Prettier leaves alone', () => {
    const rendered = renderSections([
      { heading: '✨ Added', items: ['a', 'b'] },
      { heading: '🐛 Fixed', items: ['c'] },
    ]);
    assert.equal(rendered, '### ✨ Added\n\n- a\n- b\n\n### 🐛 Fixed\n\n- c\n');
  });
});

describe('assertRenderable', () => {
  it('refuses a release with nothing in it', () => {
    assert.throws(() => assertRenderable([]), /nothing to release/);
  });

  it('refuses a heading with no bullet, like the Rust test does', () => {
    assert.throws(() => assertRenderable([{ heading: '✨ Added', items: [] }]), /empty heading/);
  });
});

const FILE = `# Changelog

Notable changes, newest first.

## [0.1.1] - 2026-08-27

### 🐛 Fixed

- Various fixes.

## [0.1.0] - 2026-07-28

### ✨ Added

- Notes and spaces.
`;

describe('splice', () => {
  it('inserts under the preamble and leaves the rest untouched', () => {
    const result = splice(FILE, '0.2.0', '## [0.2.0] - 2026-09-13\n\n### ✨ Added\n\n- A thing\n');

    assert.match(result, /^# Changelog\n\nNotable changes, newest first\.\n\n## \[0\.2\.0\]/);
    assert.ok(result.includes('## [0.1.1] - 2026-08-27'));
    assert.ok(result.includes('## [0.1.0] - 2026-07-28'));
    assert.ok(result.endsWith('- Notes and spaces.\n'));
    assert.equal(result.endsWith('\n\n'), false);
  });

  it('refuses to write a version the file already carries', () => {
    assert.throws(() => splice(FILE, '0.1.1', '## [0.1.1] - x\n'), /already carries a section/);
  });

  it('copes with a file that has no release yet', () => {
    assert.equal(splice('# Changelog\n', '0.1.0', '## [0.1.0] - x\n'), '# Changelog\n\n## [0.1.0] - x\n');
  });
});

describe('extractSection', () => {
  it('returns one release and stops at the next', () => {
    assert.equal(extractSection(FILE, '0.1.1'), '### 🐛 Fixed\n\n- Various fixes.\n');
    assert.equal(extractSection(FILE, '0.1.0'), '### ✨ Added\n\n- Notes and spaces.\n');
  });

  it('reads a heading written without brackets', () => {
    assert.equal(extractSection('## 0.1.0 - x\n\n- A thing\n', '0.1.0'), '- A thing\n');
  });

  it('throws when the version is absent', () => {
    assert.throws(() => extractSection(FILE, '9.9.9'), /no section for 9\.9\.9/);
  });
});
