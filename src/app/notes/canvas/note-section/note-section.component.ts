import { ChangeDetectionStrategy, Component, computed, inject, input, output } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { LANGUAGE_LABELS } from '@core/model/language.model';
import { NoteSection } from '@core/model/note.model';
import { NotesStore } from '@core/state/notes.store';
import { NoteActivation, NoteCardComponent } from '../note-card/note-card.component';

@Component({
  selector: 'app-note-section',
  imports: [NoteCardComponent, TranslocoPipe],
  templateUrl: './note-section.component.html',
  styleUrl: './note-section.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class NoteSectionComponent {
  private readonly notes = inject(NotesStore);

  readonly section = input.required<NoteSection>();

  readonly noteActivated = output<NoteActivation>();

  /**
   * A translation key, or a language's own label: no label kept in two places. A dated section
   * is named by its key, a gathered one by what it gathers.
   */
  protected readonly title = computed<{ key: string } | { text: string }>(() => {
    const { key, group } = this.section();
    if (group === null) return { key: `sections.${key}` };

    switch (group.group) {
      case 'priority':
        return {
          key: group.priority === 'none' ? 'sections.noPriority' : `notes.priority.${group.priority}`,
        };
      case 'kind':
        return { key: `notes.kindPlural.${group.kind}` };
      case 'language':
        return { text: LANGUAGE_LABELS[group.language] };
    }
  });

  /** The hue of a priority section's dot. */
  protected readonly priority = computed(() => {
    const group = this.section().group;
    return group?.group === 'priority' ? group.priority : null;
  });

  /** Ties the region to its heading, for screen-reader region navigation. */
  protected readonly headingId = computed(() => `section-heading-${this.section().id}`);

  protected createNote(): void {
    this.notes.createNote();
  }
}
