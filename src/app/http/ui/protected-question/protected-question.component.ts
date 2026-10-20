import { ChangeDetectionStrategy, Component, input, output } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { FolderColour } from '@core/model/folder.model';

let asked = 0;

/** What a protected environment asks before something leaves: a red line, a red button. */
@Component({
  selector: 'app-protected-question',
  imports: [TranslocoPipe],
  templateUrl: './protected-question.component.html',
  styleUrl: './protected-question.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ProtectedQuestionComponent {
  readonly question = input.required<string>();
  readonly colour = input<FolderColour | null>(null);
  readonly goLabel = input.required<string>();
  /** `-confirm`, `-send` and `-cancel` are added to it. */
  readonly testId = input.required<string>();

  readonly go = output<void>();
  readonly cancelled = output<void>();

  protected readonly id = `http-protected-question-${++asked}`;
}
