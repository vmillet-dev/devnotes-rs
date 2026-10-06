import { describe, expect, it } from 'vitest';
import { ToolHarness, renderTool } from '@testing/tool-harness';
import { SignalsToolComponent } from './signals-tool.component';
import { SIGNALS } from './signals.data';
import en from './signals.en.json';
import fr from './signals.fr.json';

describe('SignalsToolComponent', () => {
  type Harness = ToolHarness<SignalsToolComponent>;

  async function render(): Promise<Harness> {
    const harness = await renderTool(SignalsToolComponent);
    await expect.poll(() => harness.all('[data-testid="reference-row"]').length).toBe(SIGNALS.length);
    return harness;
  }

  const keys = (harness: Harness) =>
    harness.all('[data-testid="reference-row"]').map((row) => row.dataset['key']);
  const cell = (harness: Harness, name: string, selector: string) =>
    harness.element(`[data-key="${name}"] ${selector}`)?.textContent?.replace(/\s+/g, ' ').trim();

  async function search(harness: Harness, query: string): Promise<void> {
    const field = harness.element<HTMLInputElement>('[data-testid="reference-search"]');
    field.value = query;
    field.dispatchEvent(new Event('input'));
    await harness.settle();
  }

  it('reads a number as a signal’s number exactly', async () => {
    const harness = await render();

    await search(harness, '9');
    expect(keys(harness)).toEqual(['SIGKILL']);

    await search(harness, '40');
    expect(keys(harness)).toEqual(['SIGRTMIN']);

    await search(harness, '0');
    expect(keys(harness)).toEqual([]);
  });

  it.each([
    ['Ctrl+C', ['SIGINT']],
    ['sigterm', ['SIGTERM']],
    [
      'core dump',
      [
        'SIGQUIT',
        'SIGABRT',
        'SIGSEGV',
        'SIGBUS',
        'SIGFPE',
        'SIGILL',
        'SIGTRAP',
        'SIGSYS',
        'SIGXCPU',
        'SIGXFSZ',
      ],
    ],
  ])('finds %s by its name or its words', async (query, expected) => {
    const harness = await render();

    await search(harness, query);

    expect(keys(harness)).toEqual(expected);
  });

  it('says which cannot be caught, and the default action', async () => {
    const harness = await render();

    expect(
      harness.element('[data-key="SIGKILL"] [data-testid="signals-catchable"]').dataset['catchable'],
    ).toBe('false');
    expect(cell(harness, 'SIGTERM', '[data-testid="signals-catchable"]')).toBe('Oui');
    expect(cell(harness, 'SIGQUIT', '.action')).toBe('Arrête, core dump');
  });

  it('gives macOS’s number, marked where it differs from Linux’s', async () => {
    const harness = await render();

    expect(cell(harness, 'SIGUSR1', '[data-testid="signals-macos"]')).toBe('30');
    expect(harness.element('[data-key="SIGUSR1"] [data-testid="signals-macos"]').dataset['differs']).toBe(
      'true',
    );
    expect(cell(harness, 'SIGTERM', '[data-testid="signals-macos"]')).toBe('15');
    expect(harness.element('[data-key="SIGTERM"] [data-testid="signals-macos"]').dataset['differs']).toBe(
      'false',
    );
    expect(cell(harness, 'SIGPWR', '[data-testid="signals-macos"]')).toBe('—');
    expect(cell(harness, 'SIGINFO', '.number')).toBe('—');
  });

  it('empties its search on Vider', async () => {
    const harness = await render();
    await search(harness, 'kill');

    harness.tool.clear();
    await harness.settle();

    expect(keys(harness)).toHaveLength(SIGNALS.length);
  });

  it('has words in both languages for every signal', () => {
    const names = SIGNALS.map((signal) => signal.name).sort();

    expect(new Set(names).size).toBe(names.length);
    expect(Object.keys(fr.signals).sort()).toEqual(names);
    expect(Object.keys(en.signals).sort()).toEqual(names);
  });
});
