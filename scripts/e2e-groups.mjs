import { readdirSync, readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';

/**
 * CI runs the e2e scenarios in the groups of `e2e/groups.json`, three per OS, so that a pull
 * request takes six runners and not one per scenario. This is the one reader of that file: it
 * refuses a scenario left out, listed twice or unknown, and prints the matrix `e2e.yml` runs.
 */
export const GROUPS = 'e2e/groups.json';
export const SPECS = 'e2e/specs';

export function scenarioNames(directory = SPECS) {
  return readdirSync(directory)
    .filter((file) => file.endsWith('.e2e.ts'))
    .map((file) => file.slice(0, -'.e2e.ts'.length))
    .sort();
}

/** Every way the groups can disagree with the files, one line each; none means they agree. */
export function groupProblems(groups, scenarios) {
  const problems = [];
  const seen = new Map();

  for (const [group, names] of Object.entries(groups)) {
    if (!Array.isArray(names) || names.length === 0) {
      problems.push(`group ${group} lists no scenario`);
      continue;
    }
    for (const name of names) {
      if (seen.has(name)) {
        problems.push(`${name} is in group ${seen.get(name)} and in group ${group}`);
      } else {
        seen.set(name, group);
      }
      if (!scenarios.includes(name)) {
        problems.push(`${name} in group ${group} is no file of ${SPECS}`);
      }
    }
  }
  for (const name of scenarios) {
    if (!seen.has(name)) {
      problems.push(`${name} is in no group of ${GROUPS}: add it to the group with the shortest run`);
    }
  }
  return problems;
}

/** `[{ name, specs }]`, each group's scenarios in their numeric order, as `01-first-launch` needs. */
export function matrix(groups) {
  return Object.entries(groups).map(([name, specs]) => ({ name, specs: [...specs].sort() }));
}

if (process.argv[1] === fileURLToPath(import.meta.url)) {
  const groups = JSON.parse(readFileSync(GROUPS, 'utf8'));
  const problems = groupProblems(groups, scenarioNames());
  if (problems.length > 0) {
    for (const problem of problems) console.error(`::error file=${GROUPS}::${problem}`);
    process.exit(1);
  }
  console.log(JSON.stringify(matrix(groups)));
}
