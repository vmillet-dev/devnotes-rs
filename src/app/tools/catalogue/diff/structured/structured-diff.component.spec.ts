import { describe, expect, it, vi } from 'vitest';
import { JsonDiffAnswer } from '@core/model/tool-answers.model';
import { FakeToolsRepository } from '@testing/fake-tools-repository';
import { ToolHarness, renderTool } from '@testing/tool-harness';
import { StructuredDiffComponent } from './structured-diff.component';

const COMPARED: JsonDiffAnswer = {
  kind: 'compared',
  changes: [
    {
      kind: 'modified',
      path: '$.replicas',
      before: { kind: 'scalar', text: '2' },
      after: { kind: 'scalar', text: '6' },
    },
    { kind: 'added', path: '$.sentry', before: null, after: { kind: 'object', keys: 1 } },
    { kind: 'removed', path: '$.beta', before: { kind: 'scalar', text: 'true' }, after: null },
  ],
  counts: { added: 1, removed: 1, modified: 1 },
  rows: [
    { kind: 'same', path: '$', left: { number: 1, text: '{' }, right: { number: 1, text: '{' } },
    {
      kind: 'modified',
      path: '$.replicas',
      left: { number: 2, text: '  "replicas": 2,' },
      right: { number: 2, text: '  "replicas": 6,' },
    },
    { kind: 'removed', path: '$.beta', left: { number: 3, text: '  "beta": true' }, right: null },
    { kind: 'added', path: '$.sentry', left: null, right: { number: 3, text: '  "sentry": {}' } },
  ],
  rowsTruncated: false,
  patch: '[\n  { "op": "replace", "path": "/replicas", "value": 6 }\n]',
  formatA: 'json',
  formatB: 'json',
};

describe('StructuredDiffComponent', () => {
  const answer = (tools: FakeToolsRepository): void => {
    tools.diffAnswer = COMPARED;
  };

  async function compared(prepare = answer): Promise<ToolHarness<StructuredDiffComponent>> {
    const harness = await renderTool(StructuredDiffComponent, prepare);
    const type = (testid: string, text: string) => {
      const field = harness.element<HTMLTextAreaElement>(`[data-testid="${testid}"]`);
      field.value = text;
      field.dispatchEvent(new Event('input'));
    };
    type('structured-diff-a', '{"replicas":2,"beta":true}');
    await harness.type('structured-diff-b', '{"replicas":6,"sentry":{}}', 'diff_json');
    return harness;
  }

  const text = (harness: ToolHarness<StructuredDiffComponent>, testid: string) =>
    harness.element(`[data-testid="${testid}"]`)?.textContent?.replace(/\s+/g, ' ').trim() ?? '';

  it('compares both documents once both are there, key order and spaces ignored', async () => {
    const harness = await compared();

    expect(harness.tools.requestsOf('diff_json')).toEqual([
      {
        a: '{"replicas":2,"beta":true}',
        b: '{"replicas":6,"sentry":{}}',
        formatA: null,
        formatB: null,
        ignoreKeyOrder: true,
        ignoreWhitespace: true,
      },
    ]);
    expect([
      text(harness, 'structured-diff-added'),
      text(harness, 'structured-diff-removed'),
      text(harness, 'structured-diff-modified'),
    ]).toEqual(['+ 1 ajout', '− 1 suppression', '~ 1 modification']);
  });

  it('lays the rows side by side, a missing line drawn as a gap', async () => {
    const harness = await compared();

    const rows = harness.all('[data-testid="structured-diff-row"]');
    expect(rows.map((row) => row.dataset['kind'])).toEqual(['same', 'modified', 'removed', 'added']);
    expect(rows[2]!.querySelector('[data-side="b"]')!.classList).toContain('gap');
    expect(rows[3]!.querySelector('[data-side="a"]')!.classList).toContain('gap');
  });

  it('reads the same rows one after the other when unified', async () => {
    const harness = await compared();

    harness
      .element<HTMLButtonElement>(
        '[data-testid="segmented-structured-diff-layout"] [data-segment-id="unified"]',
      )
      .click();
    await harness.settle();

    expect(harness.all('[data-testid="structured-diff-line"]').map((line) => line.dataset['kind'])).toEqual([
      'same',
      'removed',
      'added',
      'removed',
      'added',
    ]);
  });

  it('lists the changes by their path, with what they were and became', async () => {
    const harness = await compared();

    const changes = harness.all('[data-testid="structured-diff-change"]');
    expect(changes.map((change) => change.dataset['path'])).toEqual(['$.replicas', '$.sentry', '$.beta']);
    const shown = (change: HTMLElement) => change.textContent?.replace(/\s+/g, ' ') ?? '';
    expect(shown(changes[0]!)).toContain('2 → 6');
    expect(shown(changes[1]!)).toContain('objet · 1 clé');

    changes[0]!.click();
    await harness.settle();
    expect(harness.element('[data-row="1"]').classList).toContain('selected');
  });

  it('copies the patch, and keeps it as a JSON note', async () => {
    const harness = await compared();

    harness.tool
      .actions()
      .find((action) => action.id === 'copy-patch')!
      .run();
    await vi.waitFor(() => expect(harness.clipboard.content).toBe(COMPARED.patch));

    expect(harness.tool.result()).toMatchObject({
      title: { key: 'tools.diff.structured.noteTitle', params: { a: 'A', b: 'B' } },
      language: 'json',
      content: COMPARED.patch,
    });
  });

  it('swaps A and B', async () => {
    const harness = await compared();

    harness.tool
      .actions()
      .find((action) => action.id === 'swap')!
      .run();
    await harness.settle();

    expect(harness.element<HTMLTextAreaElement>('[data-testid="structured-diff-a"]').value).toBe(
      '{"replicas":6,"sentry":{}}',
    );
    expect(harness.element<HTMLTextAreaElement>('[data-testid="structured-diff-b"]').value).toBe(
      '{"replicas":2,"beta":true}',
    );
  });

  it('says which side does not parse, in what format, and where', async () => {
    const harness = await compared((tools) => {
      tools.diffAnswer = { kind: 'unreadable', side: 'b', format: 'toml', line: 1, column: 12 };
    });

    expect(text(harness, 'structured-diff-unreadable-b')).toBe(
      'Ce TOML ne se lit pas : ligne 1, colonne 12.',
    );
    expect(text(harness, 'structured-diff-read-b')).toBe('lu comme TOML');
    expect(harness.element('[data-testid="structured-diff-unreadable-a"]')).toBeNull();
    expect(harness.tool.result()).toBeNull();
  });

  it('says so when the documents are the same', async () => {
    const harness = await compared((tools) => {
      tools.diffAnswer = {
        ...COMPARED,
        changes: [],
        counts: { added: 0, removed: 0, modified: 0 },
        patch: '[]',
      };
    });

    expect(harness.element('[data-testid="structured-diff-identical"]')).not.toBeNull();
    expect(harness.tool.result()).toBeNull();
  });

  it('reads each side by its shape, and says what it read', async () => {
    const harness = await compared((tools) => {
      tools.diffAnswer = { ...COMPARED, formatA: 'yaml', formatB: 'xml' };
    });

    expect([text(harness, 'structured-diff-read-a'), text(harness, 'structured-diff-read-b')]).toEqual([
      'lu comme YAML',
      'lu comme XML',
    ]);
    expect(harness.element('[data-testid="structured-diff-as-json"]')).not.toBeNull();
    expect(harness.element('[data-testid="structured-diff-yaml-note"]')).not.toBeNull();
    expect(harness.element('[data-testid="structured-diff-xml-note"]')).not.toBeNull();
  });

  it('sends a format forced on a side, and no longer says what it detected there', async () => {
    const harness = await compared();
    expect(harness.element('[data-testid="structured-diff-yaml-note"]')).toBeNull();

    harness.element<HTMLButtonElement>('[data-testid="choice-structured-diff-format-a"]').click();
    await harness.settle();
    harness
      .all('[data-testid="choice-panel-structured-diff-format-a"] [role="menuitem"]')
      .find((item) => item.textContent?.trim() === 'YAML')!
      .click();
    await harness.settle();

    await expect
      .poll(() => harness.tools.requestsOf('diff_json').at(-1))
      .toMatchObject({ formatA: 'yaml', formatB: null });
    expect(harness.element('[data-testid="structured-diff-read-a"]')).toBeNull();
    expect(text(harness, 'structured-diff-read-b')).toBe('lu comme JSON');
  });
});
