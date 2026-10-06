import { TestBed } from '@angular/core/testing';
import { TranslocoService } from '@jsverse/transloco';
import { describe, expect, it } from 'vitest';
import { ToolHarness, renderTool } from '@testing/tool-harness';
import { HttpHeadersToolComponent } from './http-headers-tool.component';
import { HTTP_HEADERS } from './http-headers.data';
import en from './http-headers.en.json';
import fr from './http-headers.fr.json';

describe('HttpHeadersToolComponent', () => {
  type Harness = ToolHarness<HttpHeadersToolComponent>;

  async function render(): Promise<Harness> {
    const harness = await renderTool(HttpHeadersToolComponent);
    await expect.poll(() => harness.all('[data-testid="reference-row"]').length).toBe(HTTP_HEADERS.length);
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

  it('says what a header does, which way it goes, an example and who defines it', async () => {
    const harness = await render();

    expect(cell(harness, 'ETag', '.does')).toContain('L’empreinte de cette version');
    expect(cell(harness, 'ETag', '.example')).toBe('"33a64df5"');
    expect(cell(harness, 'ETag', '.direction')).toBe('Réponse');
    expect(cell(harness, 'Cache-Control', '.direction')).toBe('Les deux');
    expect(cell(harness, 'Strict-Transport-Security', '.by')).toBe('RFC 6797');
  });

  it('tags what matters to security, and what is deprecated', async () => {
    const harness = await render();

    expect(
      harness.element('[data-key="Content-Security-Policy"] [data-testid="http-headers-security"]'),
    ).not.toBeNull();
    expect(harness.element('[data-key="Content-Length"] [data-testid="http-headers-security"]')).toBeNull();
    expect(harness.element('[data-key="Pragma"] [data-testid="http-headers-deprecated"]')).not.toBeNull();
    const cors = HTTP_HEADERS.filter((header) => header.name.startsWith('Access-Control-'));
    expect(cors.every((header) => header.security)).toBe(true);
  });

  it.each([
    ['cors', 'Access-Control-Allow-Origin'],
    ['cookie', 'SameSite'],
    ['cache', 'Cache-Control'],
    ['content-type', 'Content-Type'],
  ])('finds %s', async (query, expected) => {
    const harness = await render();

    await search(harness, query);

    expect(keys(harness)).toContain(expected);
    expect(keys(harness).length).toBeLessThan(HTTP_HEADERS.length);
  });

  it('reads its words again in English', async () => {
    const harness = await render();

    TestBed.inject(TranslocoService).setActiveLang('en');

    await expect.poll(() => cell(harness, 'Vary', '.does')).toContain('a cache keeps one copy per value');
    expect(cell(harness, 'Vary', '.direction')).toBe('Response');
    TestBed.inject(TranslocoService).setActiveLang('fr');
  });

  it('empties its search on Vider', async () => {
    const harness = await render();
    await search(harness, 'cors');

    harness.tool.clear();
    await harness.settle();

    expect(keys(harness)).toHaveLength(HTTP_HEADERS.length);
  });

  it('has words in both languages for every header, and one name per row', () => {
    const names = HTTP_HEADERS.map((header) => header.name.toLowerCase()).sort();

    expect(new Set(names).size).toBe(names.length);
    expect(Object.keys(fr.headers).sort()).toEqual(names);
    expect(Object.keys(en.headers).sort()).toEqual(names);
  });
});
