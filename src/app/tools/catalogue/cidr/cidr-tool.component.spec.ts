import { describe, expect, it, vi } from 'vitest';
import { CidrAnswer, CidrBlock, CidrRequest, MaskRow } from '@core/model/tool-answers.model';
import { ToolHarness, renderTool } from '@testing/tool-harness';
import { CidrToolComponent } from './cidr-tool.component';

const BLOCK: CidrBlock = {
  family: 'v4',
  cidr: '10.24.8.0/21',
  typed: '10.24.8.0',
  prefix: 21,
  hostBits: 11,
  normalised: false,
  binaryNetwork: '00001010.00011000.00001',
  binaryHost: '000.00000000',
  kind: 'private',
  range: '10.0.0.0/8',
  network: '10.24.8.0',
  mask: '255.255.248.0',
  inverseMask: '0.0.7.255',
  first: '10.24.8.1',
  last: '10.24.15.254',
  broadcast: '10.24.15.255',
  addresses: '2048',
  usable: '2046',
  forms: {
    integer: '169347072',
    expanded: '0x0A180800',
    reverse: { kind: 'zones', first: '8.24.10.in-addr.arpa', last: '15.24.10.in-addr.arpa', count: 8 },
  },
};

const mask = (prefix: number, rest: Partial<MaskRow> = {}): MaskRow => ({
  prefix,
  hostBits: 32 - prefix,
  mask: '255.255.248.0',
  wildcard: '0.0.7.255',
  addresses: String(2 ** (32 - prefix)),
  usable: String(2 ** (32 - prefix) - 2),
  networks64: null,
  ...rest,
});

const ANSWER: CidrAnswer = {
  block: BLOCK,
  problem: null,
  membership: { kind: 'inside', host: '1064' },
  splitChoices: [22, 23, 24],
  split: {
    prefix: 23,
    subnets: [
      { cidr: '10.24.8.0/23', usable: '510' },
      { cidr: '10.24.10.0/23', usable: '510' },
    ],
    more: '0',
  },
  masks: [mask(20, { mask: '255.255.240.0', wildcard: '0.0.15.255' }), mask(21)],
  plan: null,
  summary: null,
};

const V6: CidrBlock = {
  ...BLOCK,
  family: 'v6',
  cidr: '2001:db8::/32',
  typed: '2001:db8::',
  prefix: 32,
  hostBits: 96,
  kind: 'documentation',
  range: '2001:db8::/32',
  network: '2001:db8::',
  mask: null,
  inverseMask: null,
  first: '2001:db8::',
  last: '2001:db8:ffff:ffff:ffff:ffff:ffff:ffff',
  broadcast: null,
  addresses: '79228162514264337593543950336',
  usable: '79228162514264337593543950336',
  forms: {
    integer: '42540766411282592856903984951653826560',
    expanded: '2001:0db8:0000:0000:0000:0000:0000:0000',
    reverse: { kind: 'zones', first: '8.b.d.0.1.0.0.2.ip6.arpa', last: '8.b.d.0.1.0.0.2.ip6.arpa', count: 1 },
  },
};

const V6_MASKS: MaskRow[] = [
  {
    prefix: 32,
    hostBits: 96,
    mask: null,
    wildcard: null,
    addresses: '1',
    usable: '1',
    networks64: '4294967296',
  },
  {
    prefix: 112,
    hostBits: 16,
    mask: null,
    wildcard: null,
    addresses: '65536',
    usable: '65536',
    networks64: null,
  },
];

describe('CidrToolComponent', () => {
  type Harness = ToolHarness<CidrToolComponent>;

  const render = (answer: CidrAnswer = ANSWER) =>
    renderTool(CidrToolComponent, (tools) => {
      tools.cidr = answer;
    });

  const lastRequest = (harness: Harness) => harness.tools.requestsOf('describe_cidr').at(-1) as CidrRequest;
  const text = (harness: Harness, testid: string) =>
    harness.element(`[data-testid="${testid}"]`)?.textContent?.replace(/\s+/g, ' ').trim();
  const row = (harness: Harness, id: string) =>
    harness
      .element(`[data-testid="cidr-facts"] [data-row="${id}"]`)
      ?.textContent?.replace(/\s+/g, ' ')
      .trim();
  const written = (harness: Harness, id: string) =>
    harness
      .element(`[data-testid="cidr-written"] [data-row="${id}"]`)
      ?.textContent?.replace(/\s+/g, ' ')
      .trim();
  const cells = (harness: Harness, testid: string, column: number) =>
    harness
      .all(`[data-testid="${testid}"] td:nth-child(${column})`)
      .map((cell) => cell.textContent?.replace(/\s+/g, ' ').trim());

  it('asks Rust as the network and the address are typed, the family read from the text', async () => {
    const harness = await render();

    await harness.type('cidr-network', '10.24.8.0/21', 'describe_cidr');
    await harness.type('cidr-member', '10.24.12.40', 'describe_cidr');

    expect(lastRequest(harness)).toEqual({
      network: '10.24.8.0/21',
      family: null,
      member: '10.24.12.40',
      split: null,
      plan: '',
      list: '',
    });
  });

  it('forces a family on request, and goes back to Auto', async () => {
    const harness = await render();
    await harness.type('cidr-network', '10.0.0.0/8', 'describe_cidr');

    harness
      .element<HTMLButtonElement>('[data-testid="segmented-cidr-family"] [data-segment-id="v6"]')
      .click();
    await vi.waitFor(() => expect(lastRequest(harness).family).toBe('v6'));
    harness
      .element<HTMLButtonElement>('[data-testid="segmented-cidr-family"] [data-segment-id="auto"]')
      .click();
    await vi.waitFor(() => expect(lastRequest(harness).family).toBeNull());
  });

  it('draws the block: its bits cut at the prefix, what it is, and its card', async () => {
    const harness = await render();
    await harness.type('cidr-network', '10.24.8.0/21', 'describe_cidr');

    expect(text(harness, 'cidr-binary-network')).toBe('00001010.00011000.00001');
    expect(text(harness, 'cidr-binary')).toBe('00001010.00011000.00001000.00000000');
    expect(text(harness, 'cidr-kind')).toContain('Réseau privé (RFC 1918, bloc 10.0.0.0/8).');
    expect(row(harness, 'mask')).toContain('255.255.248.0');
    expect(row(harness, 'broadcast')).toContain('10.24.15.255');
    expect(row(harness, 'usable')).toContain('2 046 sur 2 048 adresses');
    expect(harness.element('[data-testid="cidr-normalised"]')).toBeNull();
  });

  it('says the address is in the network, at its host number', async () => {
    const harness = await render();
    await harness.type('cidr-network', '10.24.8.0/21', 'describe_cidr');

    expect(text(harness, 'cidr-membership')).toBe('Oui · hôte n° 1 064');
    expect(harness.element('[data-testid="cidr-membership"]').dataset['inside']).toBe('true');
  });

  it('says an address outside, and one it cannot read', async () => {
    let harness = await render({ ...ANSWER, membership: { kind: 'outside' } });
    await harness.type('cidr-network', '10.24.8.0/21', 'describe_cidr');
    expect(harness.element('[data-testid="cidr-membership"]').dataset['inside']).toBe('false');

    harness = await render({ ...ANSWER, membership: { kind: 'unreadable' } });
    await harness.type('cidr-network', '10.24.8.0/21', 'describe_cidr');
    expect(text(harness, 'cidr-membership')).toBe('Cette adresse ne se lit pas.');
  });

  it('splits on the chip chosen, and starts again from the first on a new network', async () => {
    const harness = await render();
    await harness.type('cidr-network', '10.24.8.0/21', 'describe_cidr');

    expect(harness.all('[data-testid="cidr-split-choice"]').map((chip) => chip.textContent?.trim())).toEqual([
      '/22',
      '/23',
      '/24',
    ]);
    expect(harness.all('[data-testid="cidr-subnet"]').map((subnet) => subnet.dataset['cidr'])).toEqual([
      '10.24.8.0/23',
      '10.24.10.0/23',
    ]);
    expect(text(harness, 'cidr-subnets')).toContain('510 hôtes');

    harness.element<HTMLButtonElement>('[data-testid="cidr-split-choice"][data-prefix="24"]').click();
    await vi.waitFor(() => expect(lastRequest(harness).split).toBe(24));

    await harness.type('cidr-network', '192.168.0.0/16', 'describe_cidr');
    expect(lastRequest(harness).split).toBeNull();
  });

  it('counts the subnets it does not list', async () => {
    const harness = await render({ ...ANSWER, split: { ...ANSWER.split!, more: '65280' } });
    await harness.type('cidr-network', '10.0.0.0/8', 'describe_cidr');

    expect(text(harness, 'cidr-more')).toBe('Et 65 280 de plus, non listés.');
  });

  it('draws an IPv6 block without mask nor broadcast, its count exact past 2⁵³', async () => {
    const harness = await render({
      ...ANSWER,
      block: V6,
      membership: { kind: 'empty' },
      split: null,
      masks: V6_MASKS,
    });
    await harness.type('cidr-network', '2001:db8::/32', 'describe_cidr');

    expect(row(harness, 'mask')).toBeUndefined();
    expect(row(harness, 'broadcast')).toBeUndefined();
    expect(row(harness, 'network')).toContain('Préfixe');
    expect(row(harness, 'usable')).toContain('79 228 162 514 264 337 593 543 950 336 adresses');
    expect(harness.element('[data-testid="cidr-membership"]')).toBeNull();
    expect(written(harness, 'expanded')).toContain('2001:0db8:0000');
    expect(written(harness, 'reverse')).toContain('8.b.d.0.1.0.0.2.ip6.arpa');
    expect(cells(harness, 'cidr-mask', 2)).toEqual(['2⁹⁶', '2¹⁶']);
    expect(cells(harness, 'cidr-mask', 3)).toEqual(['4 294 967 296', '—']);
  });

  it('says why a network cannot be read, and that host bits were cleared', async () => {
    let harness = await render({ ...ANSWER, block: null, problem: 'maskWithHoles' });
    await harness.type('cidr-network', '10.0.0.0 255.0.255.0', 'describe_cidr');
    expect(harness.element('[data-testid="cidr-problem"]').dataset['problem']).toBe('maskWithHoles');
    expect(harness.element('[data-testid="cidr-facts"]')).toBeNull();

    harness = await render({ ...ANSWER, block: { ...BLOCK, normalised: true } });
    await harness.type('cidr-network', '10.24.8.5/21', 'describe_cidr');
    expect(text(harness, 'cidr-normalised')).toContain('10.24.8.0/21');
  });

  it('keeps the card as a note', async () => {
    const harness = await render();
    await harness.type('cidr-network', '10.24.8.0/21', 'describe_cidr');

    const result = harness.tool.result();
    // French groups digits with a narrow no-break space.
    expect({ ...result, content: result?.content.replaceAll(' ', ' ') }).toEqual({
      title: { key: 'tools.cidr.noteTitle' },
      kind: 'snippet',
      language: 'txt',
      content: [
        '10.24.8.0/21',
        'Adresse réseau : 10.24.8.0',
        'Masque : 255.255.248.0',
        'Masque inverse : 0.0.7.255',
        'Première adresse hôte : 10.24.8.1',
        'Dernière adresse hôte : 10.24.15.254',
        'Diffusion : 10.24.15.255',
        'Hôtes utilisables : 2 046 sur 2 048 adresses',
      ].join('\n'),
    });
  });

  it('fills the mockup’s network as its sample and empties on Vider', async () => {
    const harness = await render();
    harness.tool.sample();
    await vi.waitFor(() =>
      expect(lastRequest(harness)).toMatchObject({ network: '10.24.8.0/21', member: '10.24.12.40' }),
    );

    harness.tool.clear();
    await vi.waitFor(() => expect(lastRequest(harness)).toMatchObject({ network: '', member: '' }));
  });

  it('fills a plan and a list as its sample too, and empties both', async () => {
    const harness = await render();
    harness.tool.sample();
    await vi.waitFor(() => expect(lastRequest(harness).plan).toContain('Bureaux 500'));
    expect(lastRequest(harness).list).toContain('10.24.13.5 - 10.24.13.20');

    harness.tool.clear();
    await vi.waitFor(() => expect(lastRequest(harness)).toMatchObject({ plan: '', list: '' }));
  });

  it('writes the network otherwise: one integer, in hexadecimal, and its reverse zones', async () => {
    const harness = await render();
    await harness.type('cidr-network', '10.24.8.0/21', 'describe_cidr');

    expect(written(harness, 'integer')).toContain('169347072');
    expect(written(harness, 'expanded')).toContain('0x0A180800');
    expect(written(harness, 'reverse')).toContain('8 zones, de 8.24.10.in-addr.arpa à 15.24.10.in-addr.arpa');
    expect(harness.element('[data-testid="cidr-written"] [data-row="reverse"] app-copy-value')).toBeNull();
  });

  it('names a single address’s record, and a small block’s delegated zone', async () => {
    let harness = await render({
      ...ANSWER,
      block: {
        ...BLOCK,
        forms: { ...BLOCK.forms, reverse: { kind: 'record', name: '40.12.24.10.in-addr.arpa' } },
      },
    });
    await harness.type('cidr-network', '10.24.12.40', 'describe_cidr');
    expect(written(harness, 'reverse')).toContain('40.12.24.10.in-addr.arpa');
    expect(
      harness.element('[data-testid="cidr-written"] [data-row="reverse"] app-copy-value'),
    ).not.toBeNull();

    harness = await render({
      ...ANSWER,
      block: {
        ...BLOCK,
        forms: { ...BLOCK.forms, reverse: { kind: 'classless', zone: '12.24.10.in-addr.arpa' } },
      },
    });
    await harness.type('cidr-network', '10.24.12.0/26', 'describe_cidr');
    expect(written(harness, 'reverse')).toContain(
      '12.24.10.in-addr.arpa, en délégation sans classe (RFC 2317)',
    );
  });

  it('marks the prefix typed in the masks, and applies another to the address as typed', async () => {
    const harness = await render({ ...ANSWER, block: { ...BLOCK, typed: '10.24.8.5' } });
    await harness.type('cidr-network', '10.24.8.5/21', 'describe_cidr');

    const current = harness.element('[data-testid="cidr-mask"][aria-current="true"]');
    expect(current.dataset['prefix']).toBe('21');
    expect(cells(harness, 'cidr-mask', 2)).toEqual(['255.255.240.0', '255.255.248.0']);
    expect(cells(harness, 'cidr-mask', 5)).toEqual(['4 094', '2 046']);

    harness.element<HTMLButtonElement>('[data-testid="cidr-mask"][data-prefix="20"] button').click();
    await vi.waitFor(() =>
      expect(lastRequest(harness)).toMatchObject({ network: '10.24.8.5/20', split: null }),
    );
  });

  it('lays the plan out: what was placed, what found no room, and what is left', async () => {
    const harness = await render({
      ...ANSWER,
      plan: {
        lines: [
          {
            kind: 'placed',
            name: 'Bureaux',
            hosts: '500',
            cidr: '10.24.8.0/23',
            usable: '510',
            first: '10.24.8.1',
            last: '10.24.9.254',
          },
          {
            kind: 'placed',
            name: '',
            hosts: '2',
            cidr: '10.24.10.0/31',
            usable: '2',
            first: '10.24.10.0',
            last: '10.24.10.1',
          },
          { kind: 'noRoom', name: 'Datacenter', hosts: '5000' },
          { kind: 'unreadable', text: 'Bureaux' },
        ],
        free: ['10.24.10.2/31', '10.24.10.4/30'],
      },
    });
    await harness.type('cidr-plan', 'Bureaux 500\n2\nDatacenter 5000\nBureaux', 'describe_cidr');

    expect(lastRequest(harness).plan).toBe('Bureaux 500\n2\nDatacenter 5000\nBureaux');
    expect(harness.all('[data-testid="cidr-plan-line"]').map((line) => line.dataset['kind'])).toEqual([
      'placed',
      'placed',
      'noRoom',
      'unreadable',
    ]);
    expect(cells(harness, 'cidr-plan-lines', 1).slice(0, 3)).toEqual(['Bureaux', 'Sans nom', 'Datacenter']);
    expect(cells(harness, 'cidr-plan-lines', 5)[0]).toBe('10.24.8.1 – 10.24.9.254');
    expect(text(harness, 'cidr-plan-lines')).toContain('Plus assez de place dans le réseau');
    expect(text(harness, 'cidr-plan-lines')).toContain('Ligne sans nombre d’hôtes : « Bureaux »');
    expect(text(harness, 'cidr-plan-free')).toContain('Reste libre :');
    expect(harness.all('[data-testid="cidr-plan-free"] code').map((free) => free.textContent)).toEqual([
      '10.24.10.2/31',
      '10.24.10.4/30',
    ]);
  });

  it('says a plan fills the network, and waits for a network to place one in', async () => {
    let harness = await render({ ...ANSWER, plan: { lines: [], free: [] } });
    await harness.type('cidr-network', '10.24.8.0/21', 'describe_cidr');
    expect(text(harness, 'cidr-plan-free')).toBe('Le réseau est entièrement attribué.');

    harness = await render({ ...ANSWER, block: null, problem: 'unreadable' });
    await harness.type('cidr-plan', 'Bureaux 500', 'describe_cidr');
    expect(harness.element('[data-testid="cidr-plan-waiting"]')).not.toBeNull();
  });

  it('summarises a list per family: its common block, what it adds, and its fewest blocks', async () => {
    const harness = await render({
      ...ANSWER,
      block: null,
      summary: {
        families: [
          {
            family: 'v4',
            blocks: ['10.24.8.0/22', '10.24.12.0/24'],
            supernet: '10.24.8.0/21',
            extra: '768',
          },
          { family: 'v6', blocks: ['2001:db8::/47'], supernet: '2001:db8::/47', extra: '0' },
        ],
        unreadable: ['ici'],
      },
    });
    await harness.type('cidr-list', '10.24.8.0/22\n10.24.12.0/24\n2001:db8::/47\nici', 'describe_cidr');

    const [v4, v6] = harness.all('[data-testid="cidr-summary"]');
    expect(v4.querySelector('.meta')?.textContent).toBe('768 adresses hors de la liste');
    expect(v4.querySelector('[data-testid="output-value"]')?.textContent).toBe('10.24.8.0/21');
    expect(v4.textContent?.replace(/\s+/g, ' ')).toContain('IPv4 au plus juste · 2 blocs');
    expect(v6.textContent?.replace(/\s+/g, ' ')).toContain('Exactement la liste');
    expect(
      harness.all('[data-testid="cidr-summary-block"]').map((block) => block.textContent?.trim()),
    ).toEqual(['10.24.8.0/22', '10.24.12.0/24', '2001:db8::/47']);
    expect(text(harness, 'cidr-list-unreadable')).toBe('Illisible : ici');
  });

  it('keeps the plan and the summary in the note, after the card', async () => {
    const harness = await render({
      ...ANSWER,
      plan: {
        lines: [
          {
            kind: 'placed',
            name: 'Bureaux',
            hosts: '500',
            cidr: '10.24.8.0/23',
            usable: '510',
            first: '10.24.8.1',
            last: '10.24.9.254',
          },
          { kind: 'noRoom', name: 'Datacenter', hosts: '5000' },
        ],
        free: [],
      },
      summary: {
        families: [{ family: 'v4', blocks: ['10.24.8.0/22'], supernet: '10.24.8.0/22', extra: '0' }],
        unreadable: [],
      },
    });
    await harness.type('cidr-network', '10.24.8.0/21', 'describe_cidr');

    const content = harness.tool.result()?.content.split('\n') ?? [];
    expect(content.slice(-6)).toEqual([
      'Plan d’adressage',
      'Bureaux : 10.24.8.0/23 (10.24.8.1 – 10.24.9.254)',
      '',
      'Résumer une liste',
      '10.24.8.0/22',
      'Bloc commun : 10.24.8.0/22',
    ]);
  });
});
