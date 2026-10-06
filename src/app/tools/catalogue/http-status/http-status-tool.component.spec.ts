import { TestBed } from '@angular/core/testing';
import { TranslocoService } from '@jsverse/transloco';
import { describe, expect, it } from 'vitest';
import { ToolSessions } from '@core/services/tools/tool-sessions';
import { ToolHarness, renderTool } from '@testing/tool-harness';
import { HttpStatusToolComponent } from './http-status-tool.component';
import { HTTP_STATUSES } from './http-status.data';
import en from './http-status.en.json';
import fr from './http-status.fr.json';

describe('HttpStatusToolComponent', () => {
  type Harness = ToolHarness<HttpStatusToolComponent>;

  async function render(): Promise<Harness> {
    const harness = await renderTool(HttpStatusToolComponent);
    await expect.poll(() => harness.all('[data-testid="reference-row"]').length).toBe(HTTP_STATUSES.length);
    return harness;
  }

  const keys = (harness: Harness) =>
    harness.all('[data-testid="reference-row"]').map((row) => row.dataset['key']);
  const cell = (harness: Harness, code: number, selector: string) =>
    harness.element(`[data-key="${code}"] ${selector}`)?.textContent?.replace(/\s+/g, ' ').trim();

  async function search(harness: Harness, query: string): Promise<void> {
    const field = harness.element<HTMLInputElement>('[data-testid="reference-search"]');
    field.value = query;
    field.dispatchEvent(new Event('input'));
    await harness.settle();
  }

  it('lists every code under its class, with its words in the language on screen', async () => {
    const harness = await render();

    expect(harness.all('[data-testid="reference-group"]').map((group) => group.dataset['group'])).toEqual([
      '1xx',
      '2xx',
      '3xx',
      '4xx',
      '5xx',
    ]);
    expect(cell(harness, 404, '.name')).toBe('Not Found');
    expect(cell(harness, 404, '.meaning')).toContain('Rien à cette URL.');
    expect(cell(harness, 404, '.meaning')).toContain('Quand : Une ressource inconnue');
    expect(cell(harness, 404, '.by')).toBe('RFC 9110');
  });

  it.each([
    ['404', ['404']],
    ['not found', ['404']],
    ['TEAPOT', ['418']],
    ['nginx', ['444', '494', '495', '496', '497', '499']],
  ])('finds %s', async (query, expected) => {
    const harness = await render();

    await search(harness, query);

    expect(keys(harness)).toEqual(expected);
  });

  it('finds every redirection by its heading, and by the words on screen', async () => {
    const harness = await render();

    await search(harness, 'redirection');
    expect(keys(harness)).toEqual(['300', '301', '302', '303', '304', '305', '307', '308']);

    await search(harness, 'théière');
    expect(keys(harness)).toEqual(['418']);
  });

  it('marks what is not in the registry, and what was dropped from it', async () => {
    const harness = await render();

    expect(harness.element('[data-key="499"] [data-testid="http-status-standing"]').dataset['standing']).toBe(
      'unofficial',
    );
    expect(cell(harness, 305, '[data-testid="http-status-standing"]')).toBe('Obsolète');
    expect(harness.element('[data-key="404"] [data-testid="http-status-standing"]')).toBeNull();
  });

  it('reads its words again in English, and finds by them', async () => {
    const harness = await render();

    TestBed.inject(TranslocoService).setActiveLang('en');
    await expect.poll(() => cell(harness, 404, '.meaning')).toContain('Nothing at this URL.');
    await search(harness, 'redirect');

    expect(keys(harness)).toContain('301');
    expect(harness.element('[data-group="3xx"] th')?.textContent).toContain('Redirection');
    TestBed.inject(TranslocoService).setActiveLang('fr');
  });

  it('keeps the search for the session, and Vider empties it', async () => {
    const harness = await render();
    await search(harness, '429');

    expect(TestBed.inject(ToolSessions).slot('http-status.query', '')()).toBe('429');
    expect(keys(harness)).toEqual(['429']);

    harness.tool.clear();
    await harness.settle();
    expect(keys(harness)).toHaveLength(HTTP_STATUSES.length);
    expect(harness.element<HTMLInputElement>('[data-testid="reference-search"]').value).toBe('');
  });

  it('has the same words in both languages, for every code it lists', () => {
    const codes = HTTP_STATUSES.map((status) => String(status.code)).sort();

    expect(Object.keys(fr.codes).sort()).toEqual(codes);
    expect(Object.keys(en.codes).sort()).toEqual(codes);
    for (const code of codes) {
      const [said, dit] = [en.codes[code as keyof typeof en.codes], fr.codes[code as keyof typeof fr.codes]];
      expect('use' in said, code).toBe('use' in dit);
    }
  });
});
