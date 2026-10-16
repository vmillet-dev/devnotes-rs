import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { describe, it } from 'node:test';

import { GROUPS, groupProblems, matrix, scenarioNames } from './e2e-groups.mjs';

const SCENARIOS = ['01-first-launch', '02-note-lifecycle', '03-editing-a-note'];

describe('e2e groups', () => {
  it('holds every scenario of the repository exactly once', () => {
    const groups = JSON.parse(readFileSync(GROUPS, 'utf8'));
    assert.deepEqual(groupProblems(groups, scenarioNames()), []);
  });

  it('agrees when each scenario is in one group', () => {
    const groups = { a: ['02-note-lifecycle'], b: ['01-first-launch', '03-editing-a-note'] };
    assert.deepEqual(groupProblems(groups, SCENARIOS), []);
  });

  it('refuses a scenario in no group', () => {
    const problems = groupProblems({ a: ['01-first-launch', '02-note-lifecycle'] }, SCENARIOS);
    assert.equal(problems.length, 1);
    assert.match(problems[0], /^03-editing-a-note is in no group/);
  });

  it('refuses a scenario in two groups', () => {
    const groups = {
      a: ['01-first-launch', '02-note-lifecycle'],
      b: ['02-note-lifecycle', '03-editing-a-note'],
    };
    assert.deepEqual(groupProblems(groups, SCENARIOS), ['02-note-lifecycle is in group a and in group b']);
  });

  it('refuses a name that is no spec file', () => {
    const groups = { a: [...SCENARIOS, '99-gone'] };
    assert.deepEqual(groupProblems(groups, SCENARIOS), ['99-gone in group a is no file of e2e/specs']);
  });

  it('refuses an empty group', () => {
    const groups = { a: SCENARIOS, b: [] };
    assert.deepEqual(groupProblems(groups, SCENARIOS), ['group b lists no scenario']);
  });

  it('runs each group in numeric order', () => {
    const groups = { a: ['03-editing-a-note', '01-first-launch'], b: ['02-note-lifecycle'] };
    assert.deepEqual(matrix(groups), [
      { name: 'a', specs: ['01-first-launch', '03-editing-a-note'] },
      { name: 'b', specs: ['02-note-lifecycle'] },
    ]);
  });
});
