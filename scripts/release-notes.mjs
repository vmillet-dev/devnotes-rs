#!/usr/bin/env node
// Release notes from the pull requests merged since the last tag.
//
// `generate` splices a `## [x.y.z] - DATE` section into CHANGELOG.md, `extract` reads one
// back out. The GitHub release body comes from `extract` at the tag rather than from a
// job output, so the file baked into the binary and the page on github.com cannot say two
// different things. The grammar is the one `src-tauri/src/changelog/model.rs` parses.

import { execFileSync } from 'node:child_process';
import { readFileSync, writeFileSync } from 'node:fs';
import { pathToFileURL } from 'node:url';

/**
 * The order of this table is the order of the sections on screen and the precedence
 * when a pull request wears two mapped labels.
 */
export const SECTIONS = [
  { id: 'added', heading: '✨ Added', labels: ['enhancement', 'feature'], kinds: ['feat'] },
  {
    id: 'changed',
    heading: '🔧 Changed',
    labels: ['change', 'performance'],
    kinds: ['change', 'perf'],
  },
  { id: 'fixed', heading: '🐛 Fixed', labels: ['bug'], kinds: ['fix'] },
  { id: 'removed', heading: '🗑️ Removed', labels: ['removal'], kinds: ['remove'] },
  { id: 'security', heading: '🔒 Security', labels: ['security'], kinds: ['security'] },
  {
    id: 'internal',
    heading: '🧰 Under the hood',
    labels: ['refactor', 'documentation', 'dependencies', 'chore', 'ci', 'test'],
    kinds: ['refactor', 'docs', 'test', 'chore', 'ci', 'build', 'deps'],
  },
];

/**
 * Subjects that describe the release rather than something in it. `(?![A-Za-z])` and
 * not `\b`: after `chore(release)` comes a `:`, and two non-word characters carry no word
 * boundary between them.
 */
const RELEASE_COMMIT = /^(chore\(release\)|chore: release|release:|bump version|bump to)(?![A-Za-z])/i;

/** The squash subject GitHub writes: the number is at the end, or it is not the merge. */
const PR_NUMBER = /\(#(\d+)\)\s*$/;

/** Spellings met in this history, folded onto the kind the table knows. */
const KINDS = new Map([
  ['feat', 'feat'],
  ['feature', 'feat'],
  ['fix', 'fix'],
  ['bugfix', 'fix'],
  ['hotfix', 'fix'],
  ['refactor', 'refactor'],
  ['refacto', 'refactor'],
  ['perf', 'perf'],
  ['docs', 'docs'],
  ['doc', 'docs'],
  ['test', 'test'],
  ['chore', 'chore'],
  ['ci', 'ci'],
  ['build', 'build'],
  ['deps', 'deps'],
  ['change', 'change'],
  ['remove', 'remove'],
  ['security', 'security'],
]);

/**
 * The prefix and its length, or `null` when the subject opens on prose. The word has to be
 * a known kind, or `Note: the panel is read-only` would lose its first word.
 */
function matchPrefix(subject) {
  const match = /^(?:\(\s*([A-Za-z]+)\s*\)|([A-Za-z]+)(?:\([^)]*\))?\s*:)\s*/.exec(subject);
  if (!match) {
    return null;
  }
  const kind = KINDS.get((match[1] ?? match[2]).toLowerCase());
  return kind ? { kind, length: match[0].length } : null;
}

export function kindOf(subject) {
  return matchPrefix(subject)?.kind ?? null;
}

export function parsePrNumber(subject) {
  const match = PR_NUMBER.exec(subject);
  return match ? Number(match[1]) : null;
}

export function isReleaseCommit(subject) {
  return RELEASE_COMMIT.test(subject.trim());
}

/**
 * `(refactor) Drop the DTO aliases (#56)` → `Drop the DTO aliases`. A `summary: details`
 * title is cut at the colon, since the panel renders an entry as one line of plain text.
 */
export function cleanTitle(subject) {
  const prefix = matchPrefix(subject);
  const text = (prefix ? subject.slice(prefix.length) : subject).replace(PR_NUMBER, '');
  // A title opening on a list marker would nest inside the bullet it is about to become.
  const entry = text.trim().replace(/^[-*+]\s+/, '');

  const colon = entry.indexOf(': ');
  if (colon === -1) {
    return entry;
  }
  const head = entry.slice(0, colon).trim();
  return head.split(/\s+/).length >= 3 ? head : entry;
}

/**
 * Which section an entry belongs to, and by which link of the chain — the second half is
 * what the dry-run summary prints. Pure on purpose: the caller does the GraphQL.
 */
export function sectionFor({ labels = [], issueLabels = [], title = '' }) {
  const claim = (worn) => {
    const set = new Set(worn.map((label) => label.toLowerCase()));
    return SECTIONS.find((section) => section.labels.some((label) => set.has(label)));
  };

  const byPr = claim(labels);
  if (byPr) {
    return { section: byPr.id, via: `label PR (${labels.join(', ')})` };
  }

  const byIssue = claim(issueLabels.map((entry) => entry.label));
  if (byIssue) {
    const source = issueLabels.find((entry) => byIssue.labels.includes(entry.label.toLowerCase()));
    return { section: byIssue.id, via: `label ticket #${source.issue} (${source.label})` };
  }

  const kind = kindOf(title);
  const byKind = kind ? SECTIONS.find((section) => section.kinds.includes(kind)) : undefined;
  if (byKind) {
    return { section: byKind.id, via: `préfixe (${kind})` };
  }

  return { section: 'internal', via: 'défaut' };
}

export function groupEntries(entries) {
  return SECTIONS.map((section) => ({
    heading: section.heading,
    items: entries.filter((entry) => entry.section === section.id).map((entry) => entry.text),
  })).filter((group) => group.items.length > 0);
}

/**
 * A release with no entry, or a heading with no bullet under it, turns `cargo test` red
 * once committed — and the commit this workflow makes is never seen by CI.
 */
export function assertRenderable(groups) {
  if (groups.length === 0) {
    throw new Error('nothing to release: no commit since the last tag carries a change');
  }
  for (const group of groups) {
    if (group.items.length === 0) {
      throw new Error(`empty heading: ${group.heading}`);
    }
  }
}

export function renderSections(groups) {
  return groups
    .map((group) => `### ${group.heading}\n\n${group.items.map((item) => `- ${item}\n`).join('')}`)
    .join('\n');
}

export function renderRelease({ version, date, groups }) {
  return `## [${version}] - ${date}\n\n${renderSections(groups)}`;
}

function headingOf(version) {
  return new RegExp(`^##\\s+\\[?${version.replaceAll('.', '\\.')}\\]?(\\s|$)`);
}

/** Above the first `## ` sits the preamble; the new release goes in just under it. */
export function splice(markdown, version, release) {
  const lines = markdown.split('\n');
  if (lines.some((line) => headingOf(version).test(line.trim()))) {
    throw new Error(`CHANGELOG.md already carries a section for ${version}`);
  }

  const first = lines.findIndex((line) => line.startsWith('## '));
  const head = (first === -1 ? lines : lines.slice(0, first)).join('\n').trimEnd();
  const tail = first === -1 ? '' : lines.slice(first).join('\n').trimEnd();
  // The blank lines are this function's to place, and a file ending on two newlines is
  // one Prettier would rewrite on the next lint.
  const body = release.trimEnd();
  return `${head}\n\n${body}\n${tail ? `\n${tail}\n` : ''}`;
}

/** The `### ` blocks of one release, without its `## ` heading. */
export function extractSection(markdown, version) {
  const lines = markdown.split('\n');
  const start = lines.findIndex((line) => headingOf(version).test(line.trim()));
  if (start === -1) {
    throw new Error(`no section for ${version} in the changelog`);
  }

  const rest = lines.slice(start + 1);
  const end = rest.findIndex((line) => line.startsWith('## '));
  return `${(end === -1 ? rest : rest.slice(0, end)).join('\n').trim()}\n`;
}

// Everything below talks to git and to GitHub; nothing above does, which is what makes the
// table, the chain and the splice testable without a network.

const run = (command, args) => execFileSync(command, args, { encoding: 'utf8', maxBuffer: 32 * 1024 * 1024 });

/** `gh` exits non-zero on a partial GraphQL answer; a NOT_FOUND alias is one, and survivable. */
function graphql(query) {
  try {
    return JSON.parse(run('gh', ['api', 'graphql', '-f', `query=${query}`]));
  } catch (error) {
    const payload = error.stdout?.toString();
    const parsed = payload ? JSON.parse(payload) : null;
    if (parsed?.data) {
      return parsed;
    }
    throw error;
  }
}

/** `owner/name`, from the environment in CI and from the remote everywhere else. */
function repositorySlug() {
  const slug =
    process.env.GITHUB_REPOSITORY ??
    run('gh', ['repo', 'view', '--json', 'nameWithOwner', '--jq', '.nameWithOwner']).trim();
  const [owner, name] = slug.split('/');
  if (!owner || !name) {
    throw new Error(`unreadable repository slug: ${slug}`);
  }
  return { owner, name };
}

/** The nearest release tag reachable from HEAD — `null` on a repository that has none. */
function lastTag() {
  try {
    return run('git', ['describe', '--tags', '--abbrev=0', '--match', 'v*']).trim();
  } catch {
    return null;
  }
}

function subjectsSince(tag) {
  const range = tag ? `${tag}..HEAD` : 'HEAD';
  return run('git', ['log', '--first-parent', '--no-merges', '--format=%s', range])
    .split('\n')
    .map((line) => line.trim())
    .filter(Boolean);
}

/**
 * Asked for by number rather than listed: a pull request merged into an intermediate
 * branch never lands as a commit on `main`, so any listing reports work this release does
 * not carry. `closingIssuesReferences` is GraphQL-only and is the point of the call — the
 * issues are labelled here, the pull requests almost never are.
 */
function pullRequests(numbers) {
  const index = new Map();
  if (numbers.length === 0) {
    return index;
  }

  const { owner, name } = repositorySlug();
  const fields = `
    number
    title
    labels(first: 20) { nodes { name } }
    closingIssuesReferences(first: 5) { nodes { number labels(first: 20) { nodes { name } } } }`;

  // Batched to stay under the node limit, and asked by alias so a number that is not a
  // pull request comes back as one null instead of failing the whole call.
  for (let start = 0; start < numbers.length; start += 50) {
    const batch = numbers.slice(start, start + 50);
    const query = `{ repository(owner: "${owner}", name: "${name}") {
      ${batch.map((number) => `pr${number}: pullRequest(number: ${number}) { ${fields} }`).join('\n')}
    } }`;

    for (const node of Object.values(graphql(query).data?.repository ?? {})) {
      if (node) {
        index.set(node.number, node);
      }
    }
  }

  const missing = numbers.filter((number) => !index.has(number));
  if (missing.length) {
    console.warn(
      `::warning::unreadable pull requests, falling back to the commit subject: ${missing.join(', ')}`,
    );
  }
  return index;
}

function entriesFor(subjects, index) {
  const entries = [];
  for (const subject of subjects) {
    if (isReleaseCommit(subject)) {
      continue;
    }

    const number = parsePrNumber(subject);
    const pr = number === null ? null : index.get(number);
    const title = pr ? pr.title : subject;
    // The hand-made "Bump version to 0.1.3" sits inside the range of the release it names.
    if (isReleaseCommit(title)) {
      continue;
    }

    const labels = pr ? pr.labels.nodes.map((label) => label.name) : [];
    const issueLabels = pr
      ? pr.closingIssuesReferences.nodes.flatMap((issue) =>
          issue.labels.nodes.map((label) => ({ issue: issue.number, label: label.name })),
        )
      : [];

    const { section, via } = sectionFor({ labels, issueLabels, title });
    const text = number === null ? cleanTitle(title) : `${cleanTitle(title)} (#${number})`;
    entries.push({ section, via, text, number, title });
  }
  return entries;
}

/** What the dry run is for: every entry, where it landed, and what put it there. */
function renderSummary({ version, date, since, section, entries }) {
  const rows = entries
    .map((entry) => `| ${entry.number ? `#${entry.number}` : '—'} | ${entry.text} | ${entry.via} |`)
    .join('\n');

  return [
    `## ${version} — ${date}`,
    '',
    `Depuis \`${since ?? 'le premier commit'}\`, ${entries.length} entrée(s).`,
    '',
    section,
    '',
    '### Traçabilité',
    '',
    '| PR | Entrée | Classée par |',
    '|---|---|---|',
    rows,
    '',
  ].join('\n');
}

function parseArguments(argv) {
  const options = new Map();
  for (let index = 0; index < argv.length; index += 1) {
    const argument = argv[index];
    if (!argument.startsWith('--')) {
      continue;
    }
    const key = argument.slice(2);
    const next = argv[index + 1];
    if (next === undefined || next.startsWith('--')) {
      options.set(key, true);
    } else {
      options.set(key, next);
      index += 1;
    }
  }
  return options;
}

function main() {
  const [mode, ...rest] = process.argv.slice(2);
  const options = parseArguments(rest);
  const changelog = options.get('changelog') ?? 'CHANGELOG.md';
  const version = options.get('version');

  if (typeof version !== 'string') {
    throw new Error('usage: release-notes.mjs <generate|extract> --version <x.y.z> [--write]');
  }

  if (mode === 'extract') {
    const body = extractSection(readFileSync(changelog, 'utf8'), version);
    const out = options.get('out');
    if (typeof out === 'string') {
      writeFileSync(out, body);
    }
    process.stdout.write(body);
    return;
  }

  if (mode !== 'generate') {
    throw new Error(`unknown mode: ${mode ?? '(none)'}`);
  }

  const since = options.get('since') ?? lastTag();
  const date = options.get('date') ?? new Date().toISOString().slice(0, 10);
  const subjects = subjectsSince(typeof since === 'string' ? since : null);
  const entries = entriesFor(subjects, pullRequests(subjects.map(parsePrNumber).filter(Boolean)));

  const groups = groupEntries(entries);
  assertRenderable(groups);
  const release = renderRelease({ version, date, groups });

  process.stdout.write(`${release}\n`);

  const summary = options.get('summary');
  if (typeof summary === 'string') {
    writeFileSync(summary, renderSummary({ version, date, since, section: release, entries }));
  }

  if (options.get('write') === true) {
    writeFileSync(changelog, splice(readFileSync(changelog, 'utf8'), version, release));
  }
}

// Importable by the tests without running: they need the functions, not the side effects.
if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  main();
}
