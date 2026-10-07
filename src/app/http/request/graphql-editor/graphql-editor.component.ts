import { ChangeDetectionStrategy, Component, computed, inject, input, output, resource } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { HttpRepository } from '@core/data/http.repository';
import { GraphqlParts } from '@core/model/http.model';
import { ChoiceMenuComponent, ChoiceOption } from '@shared/controls/choice-menu/choice-menu.component';

/** A GraphQL request's query and variables, the operation sent, and whether it goes as a GET. */
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

  /** What Rust reads of the document: its operations, and why the variables do not parse. */
  private readonly described = resource({
    params: () => this.graphql(),
    loader: ({ params }) => this.repository.describeGraphql(params),
  });
  protected readonly answer = computed(() => (this.described.hasValue() ? this.described.value() : null));
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

  private change(part: Partial<GraphqlParts>): void {
    this.graphqlChange.emit({ ...this.graphql(), ...part });
  }
}
