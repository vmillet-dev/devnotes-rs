import { describe, expect, it } from 'vitest';
import { ToolDefinition, favouritesAmong, foldForSearch, isRecentTool, toolMatches } from './tool.model';

describe('the tools model', () => {
  it('folds case and accents out of what is searched', () => {
    expect(foldForSearch('Générateur de Slug')).toBe('generateur de slug');
  });

  it('matches any of the texts, and everything on an empty query', () => {
    expect(toolMatches('  generat ', ['Générateurs', 'autre'])).toBe(true);
    expect(toolMatches('sha', ['Hash', 'SHA-256'])).toBe(true);
    expect(toolMatches('base64', ['Hash', 'SHA-256'])).toBe(false);
    expect(toolMatches('', [])).toBe(true);
  });

  it('lists the starred tools in the order they were starred, dropping one the catalogue lost', () => {
    const tool = (id: string): ToolDefinition => ({
      id,
      category: 'text',
      keywords: [],
      load: () => Promise.reject(new Error('never opened')),
    });
    const catalogue = [tool('case'), tool('hash'), tool('lorem')];

    expect(favouritesAmong(catalogue, ['lorem', 'gone', 'case']).map((found) => found.id)).toEqual([
      'lorem',
      'case',
    ]);
    expect(favouritesAmong(catalogue, [])).toEqual([]);
  });

  it('knows a recent tool from what a hand-edited file may hold', () => {
    expect(isRecentTool({ id: 'hash', at: '2026-09-29T10:00:00.000Z' })).toBe(true);
    expect(isRecentTool({ id: 'hash' })).toBe(false);
    expect(isRecentTool(null)).toBe(false);
    expect(isRecentTool('hash')).toBe(false);
  });
});
