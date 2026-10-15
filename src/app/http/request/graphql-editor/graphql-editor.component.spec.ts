import { ComponentFixture, TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { EMPTY_PARTS, GraphqlAnswer, GraphqlParts } from '@core/model/http.model';
import { FakeHttpRepository } from '@testing/fake-http-repository';
import { FakePrettier } from '@testing/fake-prettier';
import { provideAppTesting } from '@testing/testing.providers';
import { GraphqlEditorComponent } from './graphql-editor.component';

describe('GraphqlEditorComponent', () => {
  let fixture: ComponentFixture<GraphqlEditorComponent>;
  let http: FakeHttpRepository;
  let prettier: FakePrettier;
  let emitted: GraphqlParts[];

  /** What Rust would say of the document, the rest as for one that reads. */
  const said = (answer: Partial<GraphqlAnswer>): GraphqlAnswer => ({
    operations: [],
    variablesProblem: null,
    queryProblem: null,
    missing: [],
    unknown: [],
    ...answer,
  });
  const show = async (graphql: Partial<GraphqlParts>, answer: Partial<GraphqlAnswer>) => {
    http.graphqlAnswer = said(answer);
    fixture.componentRef.setInput('graphql', { ...EMPTY_PARTS.graphql, ...graphql });
    await fixture.whenStable();
  };

  beforeEach(async () => {
    TestBed.resetTestingModule();
    http = new FakeHttpRepository();
    prettier = new FakePrettier();
    TestBed.configureTestingModule({
      imports: [GraphqlEditorComponent],
      providers: [provideAppTesting({ httpRepository: http, prettier })],
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

    http.graphqlAnswer = said({
      operations: ['Invoices', 'Pay'],
      variablesProblem: { reason: 'unexpectedEnd', line: 1, column: 5, offset: 4 },
    });
    fixture.componentRef.setInput('graphql', {
      ...EMPTY_PARTS.graphql,
      query: 'query Invoices {} mutation Pay {}',
    });
    await vi.waitFor(() => expect(el('http-graphql-operation')).not.toBeNull());

    expect(el('http-graphql-operation').textContent).toContain('Invoices');
    expect(el('http-graphql-problem').textContent).toContain('ligne 1, colonne 5');
  });

  /** A query missing a brace was sent without a word, and answered by a 400. */
  it('says under the query which sign does not pair, and where', async () => {
    expect(el('http-graphql-query-problem')).toBeNull();

    await show(
      { query: 'query {\n  viewer {\n    login\n  }\n' },
      { queryProblem: { fault: 'unclosedBrace', line: 1, column: 7 } },
    );

    const problem = el('http-graphql-query-problem');
    expect(problem.textContent?.replace(/\s+/g, ' ').trim()).toBe(
      'Ligne 1, colonne 7 : cette accolade n’est pas refermée.',
    );
    expect(problem.dataset['fault']).toBe('unclosedBrace');
    // It warns: nothing but « Formater », which could not lay it out, is held back.
    expect(el<HTMLButtonElement>('http-graphql-format-query').disabled).toBe(true);

    await show({ query: '{ viewer { login } }' }, {});
    expect(el('http-graphql-query-problem')).toBeNull();
    expect(el<HTMLButtonElement>('http-graphql-format-query').disabled).toBe(false);
  });

  /** The variables were typed after the query, in its box: nothing said what goes where. */
  it('says what the variables are, which ones have no value, and adds them as Rust writes them', async () => {
    expect(el('http-graphql-variables-hint').textContent?.trim()).toBe(
      'Un objet JSON : une valeur pour chaque $variable de la requête.',
    );
    expect(el<HTMLTextAreaElement>('http-graphql-variables').placeholder).toBe('{ "code": "FR" }');
    expect(el('http-graphql-missing')).toBeNull();
    expect(el('http-graphql-unknown')).toBeNull();

    const query = 'query Country($code: ID!, $after: String) { country(code: $code) { name } }';
    await show({ query, variables: '{ "page": 2 }' }, { missing: ['code', 'after'], unknown: ['page'] });

    const named = (testId: string) => [
      el(testId).querySelector('span')?.textContent?.trim(),
      ...[...el(testId).querySelectorAll('.variable')].map((name) => name.textContent?.trim()),
    ];
    expect(named('http-graphql-missing')).toEqual(['Sans valeur :', '$code', '$after']);
    expect(named('http-graphql-unknown')).toEqual(['La requête ne déclare pas :', 'page']);
    expect(el('http-graphql-add').textContent?.trim()).toBe('Ajouter');

    http.variablesAdded = '{ "page": 2,\n  "code": null,\n  "after": null\n}';
    el<HTMLButtonElement>('http-graphql-add').click();
    await vi.waitFor(() => expect(emitted.at(-1)?.variables).toBe(http.variablesAdded));
    expect(http.callsOf('addGraphqlVariables')).toEqual([
      [{ ...EMPTY_PARTS.graphql, query, variables: '{ "page": 2 }' }],
    ]);
  });

  it('formats the query as GraphQL and the variables as JSON, through Prettier', async () => {
    expect(el<HTMLButtonElement>('http-graphql-format-query').disabled).toBe(true);
    expect(el<HTMLButtonElement>('http-graphql-format-variables').disabled).toBe(true);
    await show({ query: '{a{b}}', variables: '{"a":1}' }, {});

    prettier.answer = { kind: 'formatted', text: '{\n  a {\n    b\n  }\n}\n', cursor: 0, changedLines: [0] };
    el<HTMLButtonElement>('http-graphql-format-query').click();
    await vi.waitFor(() => expect(emitted.at(-1)?.query).toBe('{\n  a {\n    b\n  }\n}\n'));
    expect(prettier.requests.at(-1)).toMatchObject({ text: '{a{b}}', parser: 'graphql' });

    prettier.answer = { kind: 'formatted', text: '{ "a": 1 }\n', cursor: 0, changedLines: [0] };
    el<HTMLButtonElement>('http-graphql-format-variables').click();
    await vi.waitFor(() => expect(emitted.at(-1)?.variables).toBe('{ "a": 1 }\n'));
    expect(prettier.requests.at(-1)).toMatchObject({ text: '{"a":1}', parser: 'json' });

    // Variables that do not read are not Prettier's to lay out.
    await show(
      { variables: '{"a":' },
      { variablesProblem: { reason: 'unexpectedEnd', line: 1, column: 6, offset: 5 } },
    );
    expect(el<HTMLButtonElement>('http-graphql-format-variables').disabled).toBe(true);
  });
});
