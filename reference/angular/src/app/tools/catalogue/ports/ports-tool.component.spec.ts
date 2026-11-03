import { describe, expect, it } from 'vitest';
import { ToolHarness, renderTool } from '@testing/tool-harness';
import { PortsToolComponent } from './ports-tool.component';
import { PORTS, portRange } from './ports.data';
import en from './ports.en.json';
import fr from './ports.fr.json';

describe('PortsToolComponent', () => {
  type Harness = ToolHarness<PortsToolComponent>;

  async function render(): Promise<Harness> {
    const harness = await renderTool(PortsToolComponent);
    await expect.poll(() => harness.all('[data-testid="reference-row"]').length).toBe(PORTS.length);
    return harness;
  }

  const keys = (harness: Harness) =>
    harness.all('[data-testid="reference-row"]').map((row) => row.dataset['key']);
  const cell = (harness: Harness, port: number, selector: string) =>
    harness.element(`[data-key="${port}"] ${selector}`)?.textContent?.replace(/\s+/g, ' ').trim();

  async function search(harness: Harness, query: string): Promise<void> {
    const field = harness.element<HTMLInputElement>('[data-testid="reference-search"]');
    field.value = query;
    field.dispatchEvent(new Event('input'));
    await harness.settle();
  }

  it('groups the ports by IANA range, with their protocol, service and note', async () => {
    const harness = await render();

    expect(harness.all('[data-testid="reference-group"]').map((group) => group.dataset['group'])).toEqual([
      'well-known',
      'registered',
      'dynamic',
    ]);
    expect(cell(harness, 443, '.transport')).toBe('TCP et UDP');
    expect(cell(harness, 5432, '.service')).toBe('PostgreSQL');
    expect(cell(harness, 6379, '.note')).toContain('sans mot de passe');
  });

  it('reads a number as the start of a port', async () => {
    const harness = await render();

    await search(harness, '54');
    expect(keys(harness)).toEqual(['5432']);

    await search(harness, '80');
    expect(keys(harness)).toEqual(['80', '8000', '8080']);
  });

  it.each([
    ['redis', ['6379']],
    ['Vite', ['5173']],
    ['ssh', ['22', '1080']],
  ])('finds %s by its service or its note', async (query, expected) => {
    const harness = await render();

    await search(harness, query);

    expect(keys(harness)).toEqual(expected);
  });

  it('empties its search on Vider', async () => {
    const harness = await render();
    await search(harness, '80');

    harness.tool.clear();
    await harness.settle();

    expect(keys(harness)).toHaveLength(PORTS.length);
  });

  it('places each port in its IANA range', () => {
    expect([0, 1023, 1024, 49151, 49152, 65535].map(portRange)).toEqual([
      'well-known',
      'well-known',
      'registered',
      'registered',
      'dynamic',
      'dynamic',
    ]);
  });

  it('has a note in both languages for every port, and one row per number', () => {
    const numbers = PORTS.map((port) => String(port.number)).sort();

    expect(new Set(numbers).size).toBe(numbers.length);
    expect(Object.keys(fr.ports).sort()).toEqual(numbers);
    expect(Object.keys(en.ports).sort()).toEqual(numbers);
  });
});
