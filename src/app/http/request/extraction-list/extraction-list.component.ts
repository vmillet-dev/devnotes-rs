import { ChangeDetectionStrategy, Component, input, output } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { ExtractionRow } from '@core/model/http.model';

/** The request's extractions, among its scripts after the answer: a path into a variable each. */
@Component({
  selector: 'app-extraction-list',
  imports: [TranslocoPipe],
  templateUrl: './extraction-list.component.html',
  styleUrl: './extraction-list.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ExtractionListComponent {
  readonly rules = input.required<readonly ExtractionRow[]>();
  readonly removed = output<number>();

  protected braced(name: string): string {
    return `{{${name}}}`;
  }
}
