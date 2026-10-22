import {
  ChangeDetectionStrategy,
  Component,
  ElementRef,
  afterNextRender,
  computed,
  inject,
  input,
  output,
  signal,
} from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { DialogComponent } from '@shared/layout/dialog/dialog.component';
import { NoteKind, Placeholder } from '@core/model/note.model';
import {
  PlaceholderFieldsComponent,
  PlaceholderValue,
} from '@notes/ui/placeholder-fields/placeholder-fields.component';

/**
 * Seeded with the values already stored on the note, and what comes out is copied and
 * kept. The values leave raw: `notes::placeholder::fill` decides what an empty field is
 * worth and what is not a field at all.
 */
@Component({
  selector: 'app-placeholder-form',
  imports: [DialogComponent, PlaceholderFieldsComponent, TranslocoPipe],
  templateUrl: './placeholder-form.component.html',
  styleUrl: './placeholder-form.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class PlaceholderFormComponent {
  readonly placeholders = input.required<readonly Placeholder[]>();

  readonly submitted = output<Record<string, string>>();
  readonly rawRequested = output<void>();
  readonly cancelled = output<void>();

  /** `null` until something is typed: copying the note's values here would freeze them. */
  private readonly typed = signal<Record<string, string> | null>(null);

  protected readonly count = computed(() => this.placeholders().length);
  /** The kind of the note filled: a Note holding a field was called a snippet. */
  readonly kind = input<NoteKind>('snippet');
  protected readonly hintKey = computed(() =>
    this.kind() === 'snippet' ? 'placeholders.hint' : 'placeholders.hintNote',
  );

  protected readonly values = computed(
    () =>
      this.typed() ??
      Object.fromEntries(this.placeholders().map((placeholder) => [placeholder.name, placeholder.value])),
  );

  constructor() {
    // The first field still empty is where typing goes: Enter then copies what was typed.
    const host = inject<ElementRef<HTMLElement>>(ElementRef).nativeElement;
    afterNextRender(() => {
      const fields = [...host.querySelectorAll<HTMLInputElement>('.field-input')];
      (fields.find((field) => field.value === '') ?? fields[0])?.focus();
    });
  }

  protected setValue({ name, value }: PlaceholderValue): void {
    this.typed.set({ ...this.values(), [name]: value });
  }

  protected submit(event: Event): void {
    event.preventDefault();

    const filled: Record<string, string> = {};
    for (const placeholder of this.placeholders()) {
      filled[placeholder.name] = this.values()[placeholder.name] ?? '';
    }
    this.submitted.emit(filled);
  }
}
