import { ComponentFixture, TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { SentResponse } from '@core/model/http.model';
import { FakeHttpRepository, sentResponse } from '@testing/fake-http-repository';
import { FakeJsonRepository } from '@testing/fake-json-repository';
import { jsonView } from '@testing/json-view.fixture';
import { provideAppTesting } from '@testing/testing.providers';
import { ResponseBodyComponent } from './response-body.component';

describe('ResponseBodyComponent', () => {
  let fixture: ComponentFixture<ResponseBodyComponent>;
  let http: FakeHttpRepository;
  let json: FakeJsonRepository;

  const show = async (response: SentResponse) => {
    fixture.componentRef.setInput('response', response);
    await fixture.whenStable();
  };

  beforeEach(async () => {
    TestBed.resetTestingModule();
    http = new FakeHttpRepository();
    json = new FakeJsonRepository();
    json.view = jsonView();
    TestBed.configureTestingModule({
      imports: [ResponseBodyComponent],
      providers: [provideAppTesting({ httpRepository: http, jsonRepository: json })],
    });
    fixture = TestBed.createComponent(ResponseBodyComponent);
    fixture.componentRef.setInput('sendId', 'Login#1');
    fixture.componentRef.setInput('response', sentResponse());
    fixture.autoDetectChanges();
    await fixture.whenStable();
  });

  const el = (selector: string) =>
    (fixture.nativeElement as HTMLElement).querySelector<HTMLElement>(selector);
  const segments = () =>
    [...(fixture.nativeElement as HTMLElement).querySelectorAll<HTMLElement>('[data-segment-id]')].map(
      (segment) => segment.dataset['segmentId'],
    );
  const choose = async (id: string) => {
    el(`[data-segment-id="${id}"]`)!.click();
    await fixture.whenStable();
  };

  it('lays JSON out and colours it, shows it as it came, or explores it once asked', async () => {
    expect(segments()).toEqual(['pretty', 'raw', 'graph', 'tree']);
    expect(el('[data-testid="http-response-pretty"]')?.textContent).toContain('"data"');
    expect(el('[data-testid="http-response-pretty"] .hljs-attr')).not.toBeNull();
    expect(json.queries).toEqual([]);

    await choose('raw');
    expect(el('[data-testid="http-response-body"]')?.textContent).toBe('{"data":[]}');

    await choose('tree');
    await vi.waitFor(() => expect(el('[data-testid="http-response-explorer"]')).not.toBeNull());
    expect(json.queries[0]?.text).toBe('{"data":[]}');
  });

  it('offers no graph for a body that is not JSON, and falls back to the formatted view', async () => {
    await choose('graph');
    await show(sentResponse({ language: 'html', body: '<p>hi</p>', pretty: null }));

    expect(segments()).toEqual(['pretty', 'raw']);
    expect(el('[data-testid="http-response-pretty"]')?.textContent).toContain('<p>hi</p>');
  });

  it('shows an image Rust hands back, and only says what another binary body is', async () => {
    http.imageAnswer = 'data:image/png;base64,iVBORw==';
    const png = { enabled: true, key: 'content-type', value: 'image/png', description: '' };
    await show(sentResponse({ headers: [png], binary: true, body: '', pretty: null }));

    await vi.waitFor(() =>
      expect(el('[data-testid="http-response-image"] img')?.getAttribute('src')).toBe(
        'data:image/png;base64,iVBORw==',
      ),
    );
    expect(http.callsOf('responseImage')).toEqual([['Login#1']]);

    const zip = { ...png, value: 'application/zip' };
    await show(sentResponse({ headers: [zip], binary: true, body: '', pretty: null }));
    expect(el('[data-testid="http-response-binary"]')?.textContent).toContain('application/zip');
    expect(http.callsOf('responseImage')).toHaveLength(1);
  });

  it('says a body was cut, and that an empty one is empty', async () => {
    await show(sentResponse({ cut: true, body: 'aaa', pretty: null, language: 'txt' }));
    expect(el('[data-testid="http-response-cut"]')?.textContent?.trim()).toBe(
      'Seul le premier Mo est affiché.',
    );

    await show(sentResponse({ body: '', pretty: null }));
    expect(el('[data-testid="http-response-empty"]')).not.toBeNull();
  });
});
