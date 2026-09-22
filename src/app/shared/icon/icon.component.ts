import { ChangeDetectionStrategy, Component, computed, input } from '@angular/core';

/**
 * Paths from Lucide (https://lucide.dev, ISC licence), 24 × 24, drawn as strokes.
 *
 * ⚠️ Paths and not glyphs: `🗑` is drawn by the system's colour font and looks different on
 * every machine, `▤` barely renders in most fonts, and neither follows `color`.
 */
const PATHS = {
  sidebar: ['M5 3h14a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2z', 'M9 3v18'],
  trash: [
    'M3 6h18',
    'M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6',
    'M8 6V4c0-1 1-2 2-2h4c1 0 2 1 2 2v2',
    'M10 11v6',
    'M14 11v6',
  ],
  filter: ['M22 3H2l8 9.46V19l4 2v-8.54L22 3z'],
} as const satisfies Record<string, readonly string[]>;

export type IconName = keyof typeof PATHS;

/** Decorative by construction: the control carrying it names itself, the icon never does. */
@Component({
  selector: 'app-icon',
  template: `
    <svg
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      stroke-width="2"
      stroke-linecap="round"
      stroke-linejoin="round"
      aria-hidden="true"
      focusable="false"
    >
      @for (d of paths(); track $index) {
        <path [attr.d]="d" />
      }
    </svg>
  `,
  styles: `
    :host {
      display: inline-flex;
      flex-shrink: 0;
      width: var(--icon-size, 16px);
      height: var(--icon-size, 16px);
    }

    svg {
      width: 100%;
      height: 100%;
    }
  `,
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class IconComponent {
  readonly name = input.required<IconName>();

  protected readonly paths = computed(() => PATHS[this.name()]);
}
