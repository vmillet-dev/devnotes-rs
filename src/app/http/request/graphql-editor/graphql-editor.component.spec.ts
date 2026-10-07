import { ComponentFixture, TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { EMPTY_PARTS, GraphqlParts } from '@core/model/http.model';
import { FakeHttpRepository } from '@testing/fake-http-repository';
import { provideAppTesting } from '@testing/testing.providers';
import { GraphqlEditorComponent } from './graphql-editor.component';

describe('GraphqlEditorComponent', () => {
  let fixture: ComponentFixture<GraphqlEditorComponent>;
  let http: FakeHttpRepository;
  let emitted: GraphqlParts[];

  beforeEach(async () => {
    TestBed.resetTestingModule();
    http = new FakeHttpRepository();
    TestBed.configureTestingModule({
      imports: [GraphqlEditorComponent],
      providers: [provideAppTesting({ httpRepository: http })],
    });
    fixture = TestBed.createComponent(GraphqlEditorComponent);
    fixture.componentRef.setInput('graphql', EMPTY_PARTS.graphql);
    emitted = [];
    fixture.componentInstance.graphqlChange.subscribe((graphql) => emitted.push(graphql));
    fixture.autoDetectChanges();
    await fixture.whenStable();
  });

  const el = <E extends HTMLElement = HTMLElement>(testId: string): E =>
    (fixture.nativeElement as HTMLElement).querySelector<E>(`[data-testid="${testId}"]`)!;
  const type = (testId: string, text: string) => {
    const field = el<HTMLTextAreaElement>(testId);
    field.value = text;
    field.dispatchEvent(new Event('input'));
  };

  it('hands on the query, the variables and the GET as they change', () => {
    type('http-graphql-query', 'query A { a }');
    type('http-graphql-variables', '{"n": 1}');
    el<HTMLInputElement>('http-graphql-get').click();

    expect(emitted).toEqual([
      { ...EMPTY_PARTS.graphql, query: 'query A { a }' },
      { ...EMPTY_PARTS.graphql, variables: '{"n": 1}' },
      { ...EMPTY_PARTS.graphql, asGet: true },
    ]);
  });

  it('offers the operation to send only when the document holds several, and says why variables do not read', async () => {
    expect(el('http-graphql-operation')).toBeNull();

    http.graphqlAnswer = {
      operations: ['Invoices', 'Pay'],
      variablesProblem: { reason: 'unexpectedEnd', line: 1, column: 5, offset: 4 },
    };
    fixture.componentRef.setInput('graphql', {
      ...EMPTY_PARTS.graphql,
      query: 'query Invoices {} mutation Pay {}',
    });
    await vi.waitFor(() => expect(el('http-graphql-operation')).not.toBeNull());

    expect(el('http-graphql-operation').textContent).toContain('Invoices');
    expect(el('http-graphql-problem').textContent).toContain('ligne 1, colonne 5');
  });
});
