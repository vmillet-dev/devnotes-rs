import { TestBed } from '@angular/core/testing';
import { describe, expect, it } from 'vitest';
import { FakeToolsRepository } from '@testing/fake-tools-repository';
import { renderTool } from '@testing/tool-harness';
import { CaseToolComponent } from './case-tool.component';

describe('CaseToolComponent', () => {
  const answer = (tools: FakeToolsRepository): void => {
    tools.cases = [
      { case: 'camel', value: 'parseHttpResponse' },
      { case: 'constant', value: 'PARSE_HTTP_RESPONSE' },
    ];
  };

  it('asks Rust once the typing pauses, and lists every case it answers', async () => {
    const harness = await renderTool(CaseToolComponent, answer);

    await harness.type('case-input', 'parse HTTP response', 'convert_case');

    expect(harness.tools.requestsOf('convert_case')).toEqual(['parse HTTP response']);
    expect(harness.all('[data-testid="output-row"]').map((row) => row.dataset['name'])).toEqual([
      'camelCase',
      'CONSTANT_CASE',
    ]);
  });

  it('keeps the cases, each named in itself, as a note', async () => {
    const harness = await renderTool(CaseToolComponent, answer);
    await harness.type('case-input', 'parse HTTP response', 'convert_case');

    expect(harness.tool.result()).toEqual({
      title: { key: 'tools.case.noteTitle', params: { text: 'parse HTTP response' } },
      kind: 'snippet',
      language: 'txt',
      content: 'camelCase      parseHttpResponse\nCONSTANT_CASE  PARSE_HTTP_RESPONSE',
    });
  });

  it('names the new cases in themselves, and keeps a list converted line by line aligned', async () => {
    const harness = await renderTool(CaseToolComponent, (tools) => {
      tools.cases = [
        { case: 'dot', value: 'user.id\nhttp.server' },
        { case: 'train', value: 'User-Id\nHttp-Server' },
      ];
    });

    await harness.type('case-input', '  userId\nHTTPServer', 'convert_case');

    expect(harness.all('[data-testid="output-row"]').map((row) => row.dataset['name'])).toEqual([
      'dot.case',
      'Train-Case',
    ]);
    expect(harness.element('[data-testid="output-value"]').textContent).toBe('user.id\nhttp.server');
    expect(harness.tool.result()).toMatchObject({
      title: { key: 'tools.case.noteTitle', params: { text: 'userId' } },
      content: 'dot.case    user.id\n            http.server\nTrain-Case  User-Id\n            Http-Server',
    });
  });

  it('asks nothing of a blank text, and empties on Vider', async () => {
    const harness = await renderTool(CaseToolComponent, answer);
    await harness.type('case-input', 'parse', 'convert_case');

    harness.tool.clear();
    await harness.settle();

    expect(harness.element<HTMLTextAreaElement>('[data-testid="case-input"]').value).toBe('');
    expect(harness.tool.result()).toBeNull();
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
