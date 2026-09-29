import { describe, expect, it, vi } from 'vitest';
import { FakeToolsRepository } from '@testing/fake-tools-repository';
import { ToolHarness, renderTool } from '@testing/tool-harness';
import { UuidToolComponent } from './uuid-tool.component';

const V7 = '01922b6e-4b30-7cc4-9a5c-6f2d8e1b3a77';

describe('UuidToolComponent', () => {
  const answer = (tools: FakeToolsRepository): void => {
    tools.uuids = [V7, '01922b6e-4b31-7000-8000-000000000001'];
    tools.uuidInspection = {
      kind: 'valid',
      version: 7,
      variant: 'rfc',
      created: '2024-09-25T23:05:01.488Z',
      nil: false,
    };
  };

  const generated = (harness: ToolHarness<UuidToolComponent>) => harness.tools.requestsOf('generate_uuids');

  it('draws v4s at once, one per line', async () => {
    const harness = await renderTool(UuidToolComponent, answer);
    await vi.waitFor(() => expect(generated(harness)).toHaveLength(1));
    await harness.settle();

    expect(generated(harness)[0]).toEqual({ version: 'v4', count: 5, uppercase: false });
    expect(harness.element('[data-testid="uuid-list"]').textContent).toBe(
      `${V7}\n01922b6e-4b31-7000-8000-000000000001`,
    );
    expect(harness.tool.result()).toMatchObject({ content: `${V7}\n01922b6e-4b31-7000-8000-000000000001` });
  });

  it('asks for v7s once chosen, and draws again on request', async () => {
    const harness = await renderTool(UuidToolComponent, answer);
    await vi.waitFor(() => expect(generated(harness)).toHaveLength(1));

    harness
      .element<HTMLButtonElement>('[data-testid="segmented-uuid-version"] [data-segment-id="v7"]')
      .click();
    await vi.waitFor(() => expect(generated(harness)).toHaveLength(2));
    harness.element<HTMLButtonElement>('[data-testid="uuid-generate"]').click();
    await vi.waitFor(() => expect(generated(harness)).toHaveLength(3));

    expect(generated(harness).slice(1)).toEqual([
      { version: 'v7', count: 5, uppercase: false },
      { version: 'v7', count: 5, uppercase: false },
    ]);
  });

  it('says a v7’s version, its variant and when it was made', async () => {
    const harness = await renderTool(UuidToolComponent, answer);

    await harness.type('uuid-checked', V7, 'inspect_uuid');

    expect(harness.element('[data-testid="uuid-version"]').textContent?.trim()).toBe('v7');
    expect(harness.element('[data-testid="uuid-facts"]').textContent).toContain('RFC 9562');
    expect(harness.element('[data-testid="uuid-created"]').textContent).toBe('2024-09-25 23:05:01.488 UTC');
  });

  it('asks for the count and the case as they are set', async () => {
    const harness = await renderTool(UuidToolComponent, answer);
    await vi.waitFor(() => expect(generated(harness)).toHaveLength(1));

    await harness.type('uuid-count', '12', 'generate_uuids');
    const count = harness.element<HTMLInputElement>('[data-testid="uuid-count"]');
    count.value = '';
    count.dispatchEvent(new Event('input'));
    const uppercase = harness.element<HTMLInputElement>('[data-testid="uuid-uppercase"]');
    uppercase.checked = true;
    uppercase.dispatchEvent(new Event('change'));
    await vi.waitFor(() => expect(generated(harness)).toHaveLength(3));

    expect(generated(harness).at(-1)).toEqual({ version: 'v4', count: 12, uppercase: true });
  });

  it('gives no time for a UUID that carries none', async () => {
    const harness = await renderTool(UuidToolComponent, (tools) => {
      tools.uuidInspection = { kind: 'valid', version: 4, variant: 'rfc', created: null, nil: false };
    });

    await harness.type('uuid-checked', 'c1e32598-a35e-496e-b8b3-0e422a66ef02', 'inspect_uuid');

    expect(harness.element('[data-testid="uuid-version"]').textContent?.trim()).toBe('v4');
    expect(harness.element('[data-testid="uuid-created"]')).toBeNull();
  });

  it('empties the UUID being checked on Vider, and keeps the drawn ones', async () => {
    const harness = await renderTool(UuidToolComponent, answer);
    await harness.type('uuid-checked', V7, 'inspect_uuid');

    harness.tool.clear();
    await harness.settle();

    expect(harness.element<HTMLInputElement>('[data-testid="uuid-checked"]').value).toBe('');
    expect(harness.element('[data-testid="uuid-facts"]')).toBeNull();
    expect(harness.element('[data-testid="uuid-list"]').textContent).toContain(V7);
  });

  it('says what is not a UUID', async () => {
    const harness = await renderTool(UuidToolComponent, (tools) => {
      tools.uuidInspection = { kind: 'invalid' };
    });

    await harness.type('uuid-checked', 'not-a-uuid', 'inspect_uuid');

    expect(harness.element('[data-testid="uuid-invalid"]')).not.toBeNull();
  });
});
