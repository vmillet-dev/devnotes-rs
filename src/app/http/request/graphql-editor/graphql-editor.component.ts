import {
  ChangeDetectionStrategy,
  Component,
  computed,
  inject,
  input,
  linkedSignal,
  output,
  resource,
} from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { HttpRepository } from '@core/data/http.repository';
import { GraphqlAnswer, GraphqlParts } from '@core/model/http.model';
import { ErrorNotifier } from '@core/services/errors/error-notifier.service';
import { FormatterService } from '@core/services/format/formatter.service';
import { ChoiceMenuComponent, ChoiceOption } from '@shared/controls/choice-menu/choice-menu.component';

/**
 * A GraphQL request's query and variables, the operation sent, and whether it goes as a GET.
 * What does not pair in the query, and which variables have no value, are Rust's to read and
 * are said under each box as it is typed: they warn, and stop nothing.
 */
@Component({
  selector: 'app-graphql-editor',
  imports: [ChoiceMenuComponent, TranslocoPipe],
  templateUrl: './graphql-editor.component.html',
  styleUrl: './graphql-editor.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class GraphqlEditorComponent {
  readonly graphql = input.required<GraphqlParts>();
  readonly graphqlChange = output<GraphqlParts>();

  private readonly repository = inject(HttpRepository);
  private readonly formatter = inject(FormatterService);
  private readonly errors = inject(ErrorNotifier);

  /** What Rust reads of the document: its operations, and why the variables do not parse. */
  private readonly described = resource({
    params: () => this.graphql(),
    loader: ({ params }) => this.repository.describeGraphql(params),
  });
  /** The last answer stays while the next is read: a line under a box must not blink at each key. */
  protected readonly answer = linkedSignal<GraphqlAnswer | undefined, GraphqlAnswer | null>({
    source: () => (this.described.hasValue() ? this.described.value() : undefined),
    computation: (fresh, previous) => fresh ?? previous?.value ?? null,
  });
  protected readonly operations = computed<readonly ChoiceOption[]>(() =>
    (this.answer()?.operations ?? []).map((name) => ({ id: name, name })),
  );

  protected onQuery(event: Event): void {
    this.change({ query: (event.target as HTMLTextAreaElement).value });
  }

  protected onVariables(event: Event): void {
    this.change({ variables: (event.target as HTMLTextAreaElement).value });
  }

  protected onOperation(name: string | null): void {
    this.change({ operationName: name });
  }

  protected onGet(event: Event): void {
    this.change({ asGet: (event.target as HTMLInputElement).checked });
  }

  /** Prettier's, as for a snippet: the query as GraphQL, the variables as JSON. */
  protected async format(part: 'query' | 'variables'): Promise<void> {
    const text = this.graphql()[part];
    const answer = await this.formatter.format(text, part === 'query' ? 'graphql' : 'json', 0, '  ');
    if (answer.kind === 'formatted' && this.graphql()[part] === text) this.change({ [part]: answer.text });
  }

  /** « Ajouter »: Rust writes a `null` for each variable the operation declares and none is given. */
  protected async addMissing(): Promise<void> {
    const before = this.graphql().variables;
    const variables = await this.errors.attempt('http.failed', () =>
      this.repository.addGraphqlVariables(this.graphql()),
    );
    if (variables !== null && this.graphql().variables === before) this.change({ variables });
  }

  private change(part: Partial<GraphqlParts>): void {
    this.graphqlChange.emit({ ...this.graphql(), ...part });
  }
}
