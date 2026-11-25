#!/usr/bin/env node
// What a release would change, for a dry run to show: nothing here writes to the repository.
// `node scripts/release-preview.mjs --version x.y.z --prompt <file> --repo <owner/name>`, once
// `release-notes.mjs generate --write` has written the section into CHANGELOG.md.

import { execFileSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { parseArgs } from 'node:util';
import { pathToFileURL } from 'node:url';

/** The last release's files, under the new version. */
export function renamed(files, previous, version) {
  return files.map((file) => file.replaceAll(previous, version));
}

/** Longer than any run of backticks in `text`, so that a diff holding a fence closes nothing. */
function fenced(text, language) {
  const longest = Math.max(0, ...[...text.matchAll(/`+/g)].map(([run]) => run.length));
  const fence = '`'.repeat(Math.max(3, longest + 1));
  return `${fence}${language}\n${text.trimEnd()}\n${fence}`;
}

export function preview({ repo, previous, version, commit, subject, diff, prompt, files }) {
  const tag = `v${version}`;
  const compare = `https://github.com/${repo}/compare/v${previous}...${tag}`;
  const rows = [
    ['Version', previous ? `${previous} → **${version}**` : `**${version}**, la première`],
    ['Publié depuis', `\`${commit}\` ${subject.replaceAll('|', '\\|')}`],
    ['Commit sur `main`', `\`chore(release): ${tag}\`, \`CHANGELOG.md\` seul`],
    ['Tag', `\`${tag}\`, sur ce commit`],
    [
      'Page de la release',
      previous
        ? `la section écrite plus haut, et [la comparaison](${compare}) une fois le tag créé`
        : 'la section écrite plus haut',
    ],
    [
      'Fichiers',
      files.length ? files.map((file) => `\`${file}\``).join('<br>') : 'aucune release à comparer',
    ],
    ['Site', `la page de téléchargement passe à ${version}`],
  ];
  return [
    `# Dry run : ce que changerait la ${version}`,
    '',
    'Aucun commit, aucun tag, aucune compilation, aucune signature, aucune publication.',
    '',
    '| Quoi | Avec quoi |',
    '|---|---|',
    ...rows.map(([what, how]) => `| ${what} | ${how} |`),
    '',
    '## `CHANGELOG.md`',
    '',
    fenced(diff || '(aucun changement)', 'diff'),
    '',
    '## Fiche de mise à jour, dans l’application',
    '',
    fenced(prompt, 'markdown'),
    '',
  ].join('\n');
}

function main() {
  const { values } = parseArgs({
    options: { version: { type: 'string' }, prompt: { type: 'string' }, repo: { type: 'string' } },
  });
  for (const name of ['version', 'prompt', 'repo']) {
    if (!values[name]) {
      throw new Error(`--${name} is required`);
    }
  }
  const run = (command, args) => execFileSync(command, args, { encoding: 'utf8' }).trim();
  /** The first release has no tag before it. */
  const attempt = (command, args) => {
    try {
      return run(command, args);
    } catch {
      return '';
    }
  };

  const tag = attempt('git', ['describe', '--tags', '--abbrev=0', '--match', 'v*']);
  const previous = tag.replace(/^v/, '');
  const assets = tag
    ? run('gh', ['release', 'view', tag, '--repo', values.repo, '--json', 'assets', '--jq', '.assets[].name'])
    : '';
  process.stdout.write(
    preview({
      repo: values.repo,
      previous,
      version: values.version,
      commit: run('git', ['rev-parse', '--short=8', 'HEAD']),
      subject: run('git', ['log', '-1', '--format=%s']),
      diff: run('git', ['diff', '--unified=1', '--', 'CHANGELOG.md']),
      prompt: readFileSync(values.prompt, 'utf8'),
      files: renamed(assets.split('\n').filter(Boolean), previous, values.version),
    }),
  );
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  main();
}
