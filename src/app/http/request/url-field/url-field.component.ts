import {
  ChangeDetectionStrategy,
  Component,
  ElementRef,
  computed,
  input,
  output,
  viewChild,
} from '@angular/core';

interface Piece {
  readonly text: string;
  readonly variable: boolean;
}

/** `{{name}}`, which v0.11.0's environments resolve; until then only drawn apart. */
const VARIABLE = /(\{\{[^{}]*\}\})/;

/**
 * The URL, its `{{variables}}` lit. The input stays a real input — its caret, selection and undo
 * are the browser's — and draws transparent over a copy that carries the colours, scrolled with it.
 */
@Component({
  selector: 'app-url-field',
  templateUrl: './url-field.component.html',
  styleUrl: './url-field.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class UrlFieldComponent {
  readonly value = input.required<string>();
  readonly label = input.required<string>();
  readonly placeholder = input('');

  readonly valueChange = output<string>();

  private readonly mirror = viewChild.required<ElementRef<HTMLElement>>('mirror');

  protected readonly pieces = computed<readonly Piece[]>(() =>
    this.value()
      .split(VARIABLE)
      .filter((text) => text !== '')
      .map((text) => ({ text, variable: VARIABLE.test(text) })),
  );

  protected onInput(event: Event): void {
    const field = event.target as HTMLInputElement;
    this.valueChange.emit(field.value);
    this.follow(field);
  }

  protected follow(field: HTMLInputElement): void {
    this.mirror().nativeElement.scrollLeft = field.scrollLeft;
  }
}
