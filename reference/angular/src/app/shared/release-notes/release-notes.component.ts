import { ChangeDetectionStrategy, Component, input } from '@angular/core';
import type { ChangelogSection } from '@core/ipc/bindings';

/**
 * A release's categories and entries as Rust cut them: « Nouveautés » draws the whole file
 * with it, and the update prompt the notes the updater hands over.
 */
@Component({
  selector: 'app-release-notes',
  templateUrl: './release-notes.component.html',
  styleUrl: './release-notes.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ReleaseNotesComponent {
  readonly sections = input.required<readonly ChangelogSection[]>();
}
