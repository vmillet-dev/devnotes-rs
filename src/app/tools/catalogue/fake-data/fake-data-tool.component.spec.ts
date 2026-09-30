import { TestBed } from '@angular/core/testing';
import { TranslocoService } from '@jsverse/transloco';
import { describe, expect, it, vi } from 'vitest';
import { FakeAnswer } from '@core/model/tool-answers.model';
import { FakeToolsRepository } from '@testing/fake-tools-repository';
import { provideAppTesting } from '@testing/testing.providers';
import { ToolHarness, renderTool } from '@testing/tool-harness';
import { FakeDataToolComponent } from './fake-data-tool.component';

const ROWS: FakeAnswer = {
  seed: 42,
  columns: ['name', 'email'],
  rows: [
    ['Zoé Lefèvre', 'zoe.lefevre@example.org'],
    ['Hugo Martin', 'hugo.martin@example.com'],
  ],
  csv: 'name,email\nZoé Lefèvre,zoe.lefevre@example.org\nHugo Martin,hugo.martin@example.com',
  json: '[\n  {\n    "name": "Zoé Lefèvre"\n  }\n]',
};

describe('FakeDataToolComponent', () => {
  const answering =
    (answer: FakeAnswer) =>
    (tools: FakeToolsRepository): void => {
      tools.fakeRows = answer;
    };

  const asked = (harness: ToolHarness<FakeDataToolComponent>) => harness.tools.requestsOf('fake_data');
  const DEFAULTS = { columns: ['name', 'email', 'phone', 'address'], count: 10, locale: 'fr', seed: null };

  it('draws French rows of four columns at once, and says the seed it drew', async () => {
    const harness = await renderTool(FakeDataToolComponent, answering(ROWS));
    await vi.waitFor(() => expect(asked(harness)).toHaveLength(1));
    await harness.settle();

    expect(asked(harness)[0]).toEqual(DEFAULTS);
    expect(harness.all('[data-testid="fake-data-row"]')).toHaveLength(2);
    expect(harness.element('[data-testid="fake-data-seed-used"]').textContent).toContain('42');
    expect(harness.element('[data-testid="fake-data-harmless"]').textContent).toContain('RFC 2606');
  });

  it('draws American rows when the interface speaks English', async () => {
    TestBed.resetTestingModule();
    const tools = new FakeToolsRepository();
    tools.fakeRows = ROWS;
    TestBed.configureTestingModule({
      imports: [FakeDataToolComponent],
      providers: [provideAppTesting({ toolsRepository: tools })],
    });
    TestBed.inject(TranslocoService).setActiveLang('en');

    const fixture = TestBed.createComponent(FakeDataToolComponent);
    fixture.autoDetectChanges();
    await vi.waitFor(() => expect(tools.requestsOf('fake_data')).toHaveLength(1));

    expect(tools.requestsOf('fake_data')[0]).toMatchObject({ locale: 'en' });
  });

  it('asks for the columns, the count, the locale and the seed chosen', async () => {
    const harness = await renderTool(FakeDataToolComponent, answering(ROWS));
    await vi.waitFor(() => expect(asked(harness)).toHaveLength(1));

    const box = (column: string) =>
      harness.element<HTMLInputElement>(`[data-testid="fake-data-column-${column}"]`);
    box('phone').checked = false;
    box('phone').dispatchEvent(new Event('change'));
    box('card').checked = true;
    box('card').dispatchEvent(new Event('change'));
    await harness.type('fake-data-count', '500', 'fake_data');
    harness
      .element<HTMLButtonElement>('[data-testid="segmented-fake-data-locale"] [data-segment-id="en"]')
      .click();
    await harness.type('fake-data-seed', '7', 'fake_data');

    expect(asked(harness).at(-1)).toEqual({
      columns: ['name', 'email', 'address', 'card'],
      count: 100,
      locale: 'en',
      seed: 7,
    });
    await harness.type('fake-data-seed', 'x', 'fake_data');
    expect(asked(harness).at(-1)).toMatchObject({ seed: null });
  });

  it('keeps the seed it drew, and draws again on request', async () => {
    const harness = await renderTool(FakeDataToolComponent, answering(ROWS));
    await vi.waitFor(() => expect(asked(harness)).toHaveLength(1));
    await harness.settle();

    harness.element<HTMLButtonElement>('[data-testid="fake-data-generate"]').click();
    await vi.waitFor(() => expect(asked(harness)).toHaveLength(2));
    harness.element<HTMLButtonElement>('[data-testid="fake-data-keep-seed"]').click();
    await vi.waitFor(() => expect(asked(harness).at(-1)).toMatchObject({ seed: 42 }));
    await harness.settle();

    expect(harness.element('[data-testid="fake-data-keep-seed"]')).toBeNull();
  });

  it('asks nothing without a column, and says so', async () => {
    const harness = await renderTool(FakeDataToolComponent, answering(ROWS));
    await vi.waitFor(() => expect(asked(harness)).toHaveLength(1));

    for (const column of ['name', 'email', 'phone', 'address']) {
      const box = harness.element<HTMLInputElement>(`[data-testid="fake-data-column-${column}"]`);
      box.checked = false;
      box.dispatchEvent(new Event('change'));
    }
    await harness.settle();

    expect(harness.element('[data-testid="fake-data-no-column"]')).not.toBeNull();
    expect(harness.tool.result()).toBeNull();
  });

  it('copies a cell on a click, and the whole as CSV or JSON', async () => {
    const harness = await renderTool(FakeDataToolComponent, answering(ROWS));
    await vi.waitFor(() => expect(asked(harness)).toHaveLength(1));
    await harness.settle();

    harness.all('[data-testid="fake-data-cell"]')[1]!.click();
    await vi.waitFor(() => expect(harness.clipboard.content).toBe('zoe.lefevre@example.org'));
    await harness.settle();
    expect(harness.all('[data-testid="fake-data-cell"]')[1]!.classList).toContain('copied');

    expect(harness.tool.result()).toMatchObject({ language: 'txt', content: ROWS.csv });
    harness
      .element<HTMLButtonElement>('[data-testid="segmented-fake-data-format"] [data-segment-id="json"]')
      .click();
    await harness.settle();
    expect(harness.tool.result()).toMatchObject({
      title: { key: 'tools.fake-data.noteTitle', params: { count: 2 } },
      language: 'json',
      content: ROWS.json,
    });
  });

  it('draws anew from a random seed on Vider', async () => {
    const harness = await renderTool(FakeDataToolComponent, answering(ROWS));
    await harness.type('fake-data-seed', '7', 'fake_data');

    harness.tool.clear();
    await vi.waitFor(() => expect(asked(harness).at(-1)).toMatchObject({ seed: null }));

    expect(harness.element<HTMLInputElement>('[data-testid="fake-data-seed"]').value).toBe('');
  });
});
