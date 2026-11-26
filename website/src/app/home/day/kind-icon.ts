import { ChangeDetectionStrategy, Component, input } from '@angular/core';

export type IconName = 'list' | 'snip' | 'calc' | 'note' | 'draw' | 'http' | 'tool' | 'search' | 'check';

/** The application's own icons for a kind of note and its tools, drawn on a 24-unit grid. */
@Component({
  selector: 'app-kind-icon',
  template: `
    <svg viewBox="0 0 24 24" aria-hidden="true" focusable="false">
      @switch (name()) {
        @case ('list') {
          <svg:rect width="18" height="18" x="3" y="3" rx="2" />
          <svg:path d="m9 12 2 2 4-4" />
        }
        @case ('snip') {
          <svg:path d="m16 18 6-6-6-6M8 6l-6 6 6 6" />
        }
        @case ('calc') {
          <svg:path d="M18 7V4H6l6 8-6 8h12v-3" />
        }
        @case ('note') {
          <svg:path d="M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z" />
          <svg:path d="M14 2v4a2 2 0 0 0 2 2h4M16 13H8M16 17H8M10 9H8" />
        }
        @case ('draw') {
          <svg:path
            d="M21.2 6.8a1 1 0 0 0-4-4L3.8 16.2a2 2 0 0 0-.5.8l-1.3 4.4a.5.5 0 0 0 .6.6l4.4-1.3a2 2 0 0 0 .8-.5z"
          />
        }
        @case ('http') {
          <svg:path d="M8 3 4 7l4 4M4 7h16M16 21l4-4-4-4M20 17H4" />
        }
        @case ('tool') {
          <svg:path
            d="M14.7 6.3a1 1 0 0 0 0 1.4l1.6 1.6a1 1 0 0 0 1.4 0l3.8-3.8a6 6 0 0 1-7.9 7.9l-6.9 6.9a2.1 2.1 0 0 1-3-3l6.9-6.9a6 6 0 0 1 7.9-7.9z"
          />
        }
        @case ('search') {
          <svg:circle cx="11" cy="11" r="7" />
          <svg:path d="m21 21-4.3-4.3" />
        }
        @case ('check') {
          <svg:path d="M20 6 9 17l-5-5" />
        }
      }
    </svg>
  `,
  styles: `
    :host {
      display: contents;
    }
  `,
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class KindIcon {
  readonly name = input.required<IconName>();
}
