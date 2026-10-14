import { ChangeDetectionStrategy, Component, input } from '@angular/core';

/** A run of plain text, or a highlighted one: `['k', 'const']` is a keyword. */
export type Token = string | readonly [kind: 'k' | 's' | 'n' | 'f' | 'c' | 'fld', text: string];

export type Line = readonly Token[];

/**
 * Numbered code, as the editor draws it. The lines are data rather than markup: their
 * spaces are text, which the template compiler would collapse and a formatter reflow.
 * Every token is an element of its own, so no whitespace node is left between two.
 */
@Component({
  selector: 'app-code',
  host: { class: 'code' },
  template: `
    @for (line of lines(); track $index; let i = $index) {
      <span class="ln" [class.lit]="lit().includes(i)" [class.mk]="marked()" [class.off]="i >= shown()">{{
        i + 1
      }}</span>
      <span class="cl" [class.lit]="lit().includes(i)" [class.mk]="marked()" [class.off]="i >= shown()">
        @for (token of line; track $index) {
          <span [class]="kind(token)">{{ text(token) }}</span>
        }
      </span>
    }
  `,
  styles: `
    .ln,
    .cl {
      transition: background-color var(--motion);
    }

    .lit {
      background: rgba(var(--blue-rgb), 0.13);
    }

    .mk {
      background: rgba(var(--amber-rgb), 0.08);
    }

    /* The bar in the margin, once per line rather than once per cell. */
    .ln.lit {
      box-shadow: inset 2px 0 var(--blue);
    }

    .ln.mk {
      box-shadow: inset 3px 0 var(--amber-edge);
    }

    .off {
      visibility: hidden;
    }
  `,
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class CodeView {
  readonly lines = input.required<readonly Line[]>();
  /** The lines that belong to what is selected elsewhere. */
  readonly lit = input<readonly number[]>([]);
  /** Every line marked as just changed. */
  readonly marked = input(false);
  /** How many lines are drawn so far; the rest keep their room. */
  readonly shown = input(Number.POSITIVE_INFINITY);

  protected kind(token: Token): string {
    return typeof token === 'string' ? '' : token[0];
  }

  protected text(token: Token): string {
    return typeof token === 'string' ? token : token[1];
  }
}
