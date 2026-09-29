import { describe, expect, it, vi } from 'vitest';
import { FakeToolsRepository } from '@testing/fake-tools-repository';
import { ToolHarness, renderTool } from '@testing/tool-harness';
import { PasswordToolComponent } from './password-tool.component';

describe('PasswordToolComponent', () => {
  const answer = (tools: FakeToolsRepository): void => {
    tools.passwords = {
      kind: 'generated',
      passwords: ['k7#Qp2!mZx', 'Wn4$hT9@rB'],
      entropyBits: 131.2,
      strength: 'veryStrong',
    };
  };

  const requests = (harness: ToolHarness<PasswordToolComponent>) =>
    harness.tools.requestsOf('generate_passwords');

  async function drawn(harness: ToolHarness<PasswordToolComponent>, count: number): Promise<void> {
    await vi.waitFor(() => expect(requests(harness)).toHaveLength(count));
    await harness.settle();
  }

  it('draws at once with every set, look-alikes left out, and says how strong', async () => {
    const harness = await renderTool(PasswordToolComponent, answer);
    await drawn(harness, 1);

    expect(requests(harness)[0]).toEqual({
      length: 20,
      sets: ['lowercase', 'uppercase', 'digits', 'symbols'],
      avoidLookAlikes: true,
      count: 5,
    });
    expect(harness.all('[data-testid="output-value"]').map((value) => value.textContent)).toEqual([
      'k7#Qp2!mZx',
      'Wn4$hT9@rB',
    ]);
    expect(harness.element('[data-testid="password-strength"]').textContent).toContain('131 bits');
    expect(harness.element('[data-testid="password-strength"]').textContent).toContain('très fort');
  });

  it('draws again on Générer, with the same options', async () => {
    const harness = await renderTool(PasswordToolComponent, answer);
    await drawn(harness, 1);

    harness.element<HTMLButtonElement>('[data-testid="password-generate"]').click();
    await drawn(harness, 2);

    expect(requests(harness)[1]).toEqual(requests(harness)[0]);
  });

  it('keeps the sets in their listed order, whichever is ticked', async () => {
    const harness = await renderTool(PasswordToolComponent, answer);
    await drawn(harness, 1);
    const digits = harness.element<HTMLInputElement>('[data-testid="password-set-digits"]');

    digits.checked = false;
    digits.dispatchEvent(new Event('change'));
    await drawn(harness, 2);
    digits.checked = true;
    digits.dispatchEvent(new Event('change'));
    await drawn(harness, 3);

    expect(requests(harness).map((request) => (request as { sets: string[] }).sets)).toEqual([
      ['lowercase', 'uppercase', 'digits', 'symbols'],
      ['lowercase', 'uppercase', 'symbols'],
      ['lowercase', 'uppercase', 'digits', 'symbols'],
    ]);
  });

  /** Your choice: a password may be kept, but the dialog says why it had better not be. */
  it('may be kept as a note, with a warning the dialog shows', async () => {
    const harness = await renderTool(PasswordToolComponent, answer);
    await drawn(harness, 1);

    expect(harness.tool.result()).toEqual({
      title: { key: 'tools.password.noteTitle' },
      kind: 'snippet',
      language: 'txt',
      content: 'k7#Qp2!mZx\nWn4$hT9@rB',
      warning: { key: 'tools.password.saveWarning' },
    });
  });

  it('says a password needs a set of characters', async () => {
    const harness = await renderTool(PasswordToolComponent, (tools) => {
      tools.passwords = { kind: 'noCharacters' };
    });
    await drawn(harness, 1);

    expect(harness.element('[data-testid="password-problem"]')).not.toBeNull();
    expect(harness.tool.result()).toBeNull();
  });
});
