import { TestBed } from '@angular/core/testing';
import { describe, expect, it, vi } from 'vitest';
import { CaseAnswer } from '@core/model/tool-answers.model';
import { FakeToolsRepository } from '@testing/fake-tools-repository';
import { renderTool } from '@testing/tool-harness';
import { CaseToolComponent } from './case-tool.component';

const ANSWER: CaseAnswer = {
  conversions: [
    { case: 'camel', group: 'code', value: 'parseHttpResponse' },
    { case: 'constant', group: 'code', value: 'PARSE_HTTP_RESPONSE' },
    { case: 'title', group: 'text', value: 'Parse Http Response' },
    { case: 'upper', group: 'text', value: 'PARSE HTTP RESPONSE' },
  ],
  words: ['parse', 'HTTP', 'response'],
  slug: 'parse-http-response',
};

describe('CaseToolComponent', () => {
  const answer = (tools: FakeToolsRepository): void => {
    tools.caseAnswer = ANSWER;
  };

  const names = (harness: { all(selector: string): HTMLElement[] }, group: 'code' | 'text' | 'url') => {
    const rows = harness.all('[data-testid="case-results"] > *');
    const start = rows.findIndex((row) => row.dataset['group'] === group);
    const after = rows.slice(start + 1);
    const end = after.findIndex((row) => row.dataset['testid'] === 'case-group');
    return (end === -1 ? after : after.slice(0, end)).map(
      (row) => row.querySelector<HTMLElement>('[data-testid="output-row"]')?.dataset['name'],
    );
  };

  it('asks Rust once the typing pauses, with the options, and lists the cases in two groups', async () => {
    const harness = await renderTool(CaseToolComponent, answer);

    await harness.type('case-input', 'parse HTTP response', 'convert_case');

    expect(harness.tools.requestsOf('convert_case')).toEqual([
      {
        text: 'parse HTTP response',
        stripAccents: true,
        perLine: true,
        titleCaseLanguage: 'english',
        slug: { separator: 'dash', lowercase: true },
      },
    ]);
    expect(names(harness, 'code')).toEqual(['camelCase', 'CONSTANT_CASE']);
    // The two named by a word of the language are named in it.
    expect(names(harness, 'text')).toEqual(['Title Case', 'MAJUSCULES']);
    expect(names(harness, 'url')).toEqual(['Slug']);
    expect(harness.element('[data-name="Slug"] [data-testid="output-value"]').textContent).toBe(
      'parse-http-response',
    );
  });

  /** One call: the slug's options are part of the case request, and Rust answers both. */
  it("asks again with the slug's separator and case", async () => {
    const harness = await renderTool(CaseToolComponent, answer);
    await harness.type('case-input', 'Été 2026', 'convert_case');
    const asked = () => harness.tools.requestsOf('convert_case').at(-1);

    harness.element('[data-testid="segmented-case-slug-separator"] [data-segment-id="underscore"]').click();
    await vi.waitFor(() =>
      expect(asked()).toMatchObject({ slug: { separator: 'underscore', lowercase: true } }),
    );

    harness.element<HTMLInputElement>('[data-testid="case-slug-lowercase"]').click();
    await vi.waitFor(() =>
      expect(asked()).toMatchObject({ slug: { separator: 'underscore', lowercase: false } }),
    );
  });

  it('shows the words the converters work from, and none before there are some', async () => {
    const harness = await renderTool(CaseToolComponent, answer);
    expect(harness.element('[data-testid="case-words"]')).toBeNull();

    await harness.type('case-input', 'parse HTTP response', 'convert_case');

    expect(harness.all('[data-testid="case-words"] li').map((chip) => chip.textContent)).toEqual([
      'parse',
      'HTTP',
      'response',
    ]);
  });

  it('asks again with each option changed', async () => {
    const harness = await renderTool(CaseToolComponent, answer);
    await harness.type('case-input', 'été', 'convert_case');
    const asked = () => harness.tools.requestsOf('convert_case').at(-1);

    harness.element<HTMLInputElement>('[data-testid="case-strip-accents"]').click();
    await vi.waitFor(() => expect(asked()).toMatchObject({ stripAccents: false }));

    harness.element<HTMLInputElement>('[data-testid="case-per-line"]').click();
    await vi.waitFor(() => expect(asked()).toMatchObject({ perLine: false }));

    harness.element('[data-testid="segmented-case-title-language"] [data-segment-id="french"]').click();
    await vi.waitFor(() => expect(asked()).toMatchObject({ titleCaseLanguage: 'french' }));
  });

  it('keeps the cases, each named in itself, as a note', async () => {
    const harness = await renderTool(CaseToolComponent, answer);
    await harness.type('case-input', 'parse HTTP response', 'convert_case');

    expect(harness.tool.result()).toEqual({
      title: { key: 'tools.case.noteTitle', params: { text: 'parse HTTP response' } },
      kind: 'snippet',
      language: 'txt',
      content:
        'camelCase      parseHttpResponse\nCONSTANT_CASE  PARSE_HTTP_RESPONSE\n' +
        'Title Case     Parse Http Response\nUPPERCASE      PARSE HTTP RESPONSE\n' +
        'slug           parse-http-response',
    });
  });

  it('keeps a list converted line by line aligned in the note', async () => {
    const harness = await renderTool(CaseToolComponent, (tools) => {
      tools.caseAnswer = {
        conversions: [
          { case: 'dot', group: 'code', value: 'user.id\nhttp.server' },
          { case: 'train', group: 'code', value: 'User-Id\nHttp-Server' },
        ],
        words: ['user', 'Id'],
        slug: 'userid\nhttpserver',
      };
    });

    await harness.type('case-input', '  userId\nHTTPServer', 'convert_case');

    expect(harness.element('[data-testid="output-value"]').textContent).toBe('user.id\nhttp.server');
    expect(harness.tool.result()).toMatchObject({
      title: { key: 'tools.case.noteTitle', params: { text: 'userId' } },
      content:
        'dot.case    user.id\n            http.server\nTrain-Case  User-Id\n            Http-Server\n' +
        'slug        userid\n            httpserver',
    });
  });

  it('asks nothing of a blank text, and empties on Vider', async () => {
    const harness = await renderTool(CaseToolComponent, answer);
    await harness.type('case-input', 'parse', 'convert_case');

    harness.tool.clear();
    await harness.settle();

    expect(harness.element<HTMLTextAreaElement>('[data-testid="case-input"]').value).toBe('');
    expect(harness.tool.result()).toBeNull();
    expect(harness.element('[data-testid="case-results"]')).toBeNull();
  });

  it('fills its sample in the language on screen', async () => {
    const harness = await renderTool(CaseToolComponent, answer);

    harness.tool.sample();
    await harness.settle();

    expect(harness.element<HTMLTextAreaElement>('[data-testid="case-input"]').value).toContain('HTTPServer');
  });

  /** Left for another tool and found again: the session keeps what was typed. */
  it('is found as it was left', async () => {
    const first = await renderTool(CaseToolComponent, answer);
    await first.type('case-input', 'kept', 'convert_case');
    first.fixture.destroy();

    const again = TestBed.createComponent(CaseToolComponent);
    again.autoDetectChanges();
    await again.whenStable();

    expect(again.nativeElement.querySelector('[data-testid="case-input"]').value).toBe('kept');
  });
});
