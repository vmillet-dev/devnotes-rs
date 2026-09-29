import { expect } from '@wdio/globals';

import { canvas } from '../pageobjects/canvas.page.js';
import { bridge } from '../support/bridge.js';

/**
 * What the visualiser draws is decided in Rust: a text in, nodes placed out. Through the
 * assembled application's bridge, since the command answers without a library lock.
 */
describe('Exploring a JSON document', () => {
  const EVENT = JSON.stringify({
    id: 'evt_1PqX4c',
    type: 'invoice.paid',
    data: { object: { amount_paid: 4900, lines: [{ description: 'Pro · mensuel' }] } },
  });

  before(async () => {
    await canvas.open();
  });

  it('answers the nodes, their rows and where each one sits', async () => {
    const view = await bridge.exploreJson({ text: EVENT, search: '', opening: { kind: 'initial' } });

    expect(view.error).toBeNull();
    expect(view.graph.nodes.map((node) => node.path)).toEqual([
      '$',
      '$.data',
      '$.data.object',
      '$.data.object.lines',
      '$.data.object.lines[0]',
    ]);
    const root = view.graph.nodes[0]!;
    expect(root.rows.map((row) => row.value)).toEqual(['"evt_1PqX4c"', '"invoice.paid"', '{ … }']);
    expect(EVENT.slice(root.rows[0]!.span.start, root.rows[0]!.span.end)).toBe('"evt_1PqX4c"');
    expect(view.graph.nodes[1]!.x).toBeGreaterThan(root.x + root.width);
    expect(view.stats).toEqual({ keys: 7, depth: 4, bytes: new TextEncoder().encode(EVENT).length });
  });

  it('finds a key or a value, folded like every search', async () => {
    const view = await bridge.exploreJson({ text: EVENT, search: 'MENSUEL', opening: { kind: 'initial' } });

    expect(view.matches).toEqual(['$.data.object.lines[0].description']);
  });

  it('says where a broken document breaks', async () => {
    const view = await bridge.exploreJson({ text: '{\n  "a": 1,,\n}', search: '', opening: { kind: 'all' } });

    expect(view.error).toEqual({ reason: 'unexpectedCharacter', line: 2, column: 10, offset: 11 });
    expect(view.graph.nodes).toEqual([]);
  });
});
