import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { describe, it } from 'node:test';

import { GROUPS, groupProblems, matrix, scenarioNames } from './e2e-groups.mjs';

const SCENARIOS = ['01_first_launch', '02_note_lifecycle', '03_editing_a_note'];

describe('e2e groups', () => {
  it('holds every scenario of the repository exactly once', () => {
    const groups = JSON.parse(readFileSync(GROUPS, 'utf8'));
    assert.deepEqual(groupProblems(groups, scenarioNames()), []);
  });

  it('agrees when each scenario is in one group', () => {
    const groups = { a: ['02_note_lifecycle'], b: ['01_first_launch', '03_editing_a_note'] };
    assert.deepEqual(groupProblems(groups, SCENARIOS), []);
  });

  it('refuses a scenario in no group', () => {
    const problems = groupProblems({ a: ['01_first_launch', '02_note_lifecycle'] }, SCENARIOS);
    assert.equal(problems.length, 1);
    assert.match(problems[0], /^03_editing_a_note is in no group/);
  });

  it('refuses a scenario in two groups', () => {
    const groups = {
      a: ['01_first_launch', '02_note_lifecycle'],
      b: ['02_note_lifecycle', '03_editing_a_note'],
    };
    assert.deepEqual(groupProblems(groups, SCENARIOS), ['02_note_lifecycle is in group a and in group b']);
  });

  it('refuses a name that is no spec file', () => {
    const groups = { a: [...SCENARIOS, '99_gone'] };
    assert.deepEqual(groupProblems(groups, SCENARIOS), ['99_gone in group a is no file of crates/e2e/tests']);
  });

  it('refuses an empty group', () => {
    const groups = { a: SCENARIOS, b: [] };
    assert.deepEqual(groupProblems(groups, SCENARIOS), ['group b lists no scenario']);
  });

  it('runs each group in numeric order', () => {
    const groups = { a: ['03_editing_a_note', '01_first_launch'], b: ['02_note_lifecycle'] };
    assert.deepEqual(matrix(groups), [
      { name: 'a', specs: ['01_first_launch', '03_editing_a_note'] },
      { name: 'b', specs: ['02_note_lifecycle'] },
    ]);
  });
});
