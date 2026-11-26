import { ChangeDetectionStrategy, Component, input } from '@angular/core';

/** A run of a sentence: plain, strong, keys pressed together, or code. */
export type Run =
  string | { readonly b: string } | { readonly keys: readonly string[] } | { readonly code: string };

/**
 * A sentence written as runs, so a dictionary can hold its keys and its code without markup. A
 * plain run is wrapped: alone in its block, the template compiler would pad it with a space, and a
 * full stop after a strong word would stand apart.
 */
@Component({
  selector: 'app-runs',
  template: `
    @for (run of runs(); track $index) {
      @if (isText(run)) {
        <ng-container>{{ run }}</ng-container>
      } @else if ('b' in run) {
        <strong>{{ run.b }}</strong>
      } @else if ('keys' in run) {
        <span class="keys">
          @for (key of run.keys; track key) {
            <kbd>{{ key }}</kbd>
          }
        </span>
      } @else {
        <code>{{ run.code }}</code>
      }
    }
  `,
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class Runs {
  readonly runs = input.required<readonly Run[]>();

  protected isText(run: Run): run is string {
    return typeof run === 'string';
  }
}
