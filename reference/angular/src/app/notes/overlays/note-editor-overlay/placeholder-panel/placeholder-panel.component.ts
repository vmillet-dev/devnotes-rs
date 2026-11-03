import {
  ChangeDetectionStrategy,
  Component,
  computed,
  input,
  linkedSignal,
  output,
  untracked,
} from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { Placeholder } from '@core/model/note.model';
import { CopyButtonComponent } from '@notes/ui/copy-button/copy-button.component';
import {
  PlaceholderFieldsComponent,
  PlaceholderValue,
} from '@notes/ui/placeholder-fields/placeholder-fields.component';

const SUMMARY_LIMIT = 2;

function storedValues(placeholders: readonly Placeholder[]): Record<string, string> {
  return Object.fromEntries(placeholders.map((placeholder) => [placeholder.name, placeholder.value]));
}

function sameValues(a: Record<string, string>, b: Record<string, string>): boolean {
  const names = new Set([...Object.keys(a), ...Object.keys(b)]);
  return [...names].every((name) => (a[name] ?? '') === (b[name] ?? ''));
}

/**
 * Holds a local draft of the values, committed on field exit. Mutates nothing and
 * fills nothing: it emits, `NotesStore` persists.
 */
@Component({
  selector: 'app-placeholder-panel',
  imports: [CopyButtonComponent, PlaceholderFieldsComponent, TranslocoPipe],
  templateUrl: './placeholder-panel.component.html',
  styleUrl: './placeholder-panel.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class PlaceholderPanelComponent {
  readonly placeholders = input.required<readonly Placeholder[]>();

  /**
   * The editor session, not the note's id: materialising a draft changes that id for
   * the same note, and a draft keyed on it is replayed over what was just typed.
   */
  readonly session = input.required<number>();

  readonly open = input(true);

  readonly previewing = input(false);

  readonly rawContent = input('');

  readonly toggled = output<void>();
  readonly previewToggled = output<void>();
  /** On every keystroke: what the preview follows. Writes nothing. */
  readonly valuesChanged = output<Record<string, string>>();
  /** On field exit: what the note stores. */
  readonly valuesCommitted = output<Record<string, string>>();

  private readonly draft = linkedSignal({
    source: this.session,
    computation: () => untracked(() => storedValues(this.placeholders())),
  });

  readonly values = this.draft.asReadonly();

  /** Against the draft: between the commit and the answer the note still carries the old values. */
  private readonly committed = linkedSignal({
    source: this.session,
    computation: () => untracked(() => storedValues(this.placeholders())),
  });

  protected readonly total = computed(() => this.placeholders().length);
  protected readonly filled = computed(
    () => this.placeholders().filter((placeholder) => this.draft()[placeholder.name]).length,
  );

  private readonly summary = computed(() =>
    this.placeholders()
      .map((placeholder) => ({ name: placeholder.name, value: this.draft()[placeholder.name] ?? '' }))
      .filter((entry) => entry.value !== ''),
  );

  protected readonly visibleSummary = computed(() => this.summary().slice(0, SUMMARY_LIMIT));
  protected readonly hiddenSummary = computed(() => Math.max(0, this.summary().length - SUMMARY_LIMIT));

  protected onChanged({ name, value }: PlaceholderValue): void {
    this.draft.update((current) => ({ ...current, [name]: value }));
    this.valuesChanged.emit(this.draft());
  }

  protected reset(): void {
    this.draft.set({});
    this.valuesChanged.emit(this.draft());
    this.commit();
  }

  /**
   * Called on field exit and by the editor before it closes: neither Escape, the
   * backdrop nor the close button produces a `blur`.
   */
  commit(): void {
    const values = Object.fromEntries(
      this.placeholders().map((placeholder) => [placeholder.name, this.draft()[placeholder.name] ?? '']),
    );

    if (sameValues(values, this.committed())) return;

    this.committed.set(values);
    this.valuesCommitted.emit(values);
  }
}
