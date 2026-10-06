import { describe, expect, it, vi } from 'vitest';
import { CidrAnswer, CidrBlock, CidrRequest } from '@core/model/tool-answers.model';
import { ToolHarness, renderTool } from '@testing/tool-harness';
import { CidrToolComponent } from './cidr-tool.component';

const BLOCK: CidrBlock = {
  family: 'v4',
  cidr: '10.24.8.0/21',
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
};

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
};

const V6: CidrBlock = {
  ...BLOCK,
  family: 'v6',
  cidr: '2001:db8::/32',
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
};

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
    harness.element(`[data-row="${id}"]`)?.textContent?.replace(/\s+/g, ' ').trim();

  it('asks Rust as the network and the address are typed, the family read from the text', async () => {
    const harness = await render();

    await harness.type('cidr-network', '10.24.8.0/21', 'describe_cidr');
    await harness.type('cidr-member', '10.24.12.40', 'describe_cidr');

    expect(lastRequest(harness)).toEqual({
      network: '10.24.8.0/21',
      family: null,
      member: '10.24.12.40',
      split: null,
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
    const harness = await render({ ...ANSWER, block: V6, membership: { kind: 'empty' }, split: null });
    await harness.type('cidr-network', '2001:db8::/32', 'describe_cidr');

    expect(row(harness, 'mask')).toBeUndefined();
    expect(row(harness, 'broadcast')).toBeUndefined();
    expect(row(harness, 'network')).toContain('Préfixe');
    expect(row(harness, 'usable')).toContain('79 228 162 514 264 337 593 543 950 336 adresses');
    expect(harness.element('[data-testid="cidr-membership"]')).toBeNull();
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
});
