import { TestBed } from '@angular/core/testing';
import { describe, expect, it } from 'vitest';
import { Tool } from '@core/services/tools/tool.model';
import { renderTool } from '@testing/tool-harness';
import { TOOLS } from './catalogue';

describe('TOOLS', () => {
  /** What a sample fills reaches Rust as typing would: the tool asks with it. */
  it.each(TOOLS.map((tool) => [tool.id, tool] as const))(
    '%s offers a sample that the tool asks Rust with',
    async (_, tool) => {
      const harness = await renderTool<Tool>(await tool.load());
      expect(typeof harness.tool.sample).toBe('function');
      const before = JSON.stringify(harness.tools.asked);

      harness.tool.sample!();
      await harness.settle();

      await expect.poll(() => JSON.stringify(harness.tools.asked), { timeout: 2_000 }).not.toBe(before);
      TestBed.resetTestingModule();
    },
  );
});
