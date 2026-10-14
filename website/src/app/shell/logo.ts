import { ChangeDetectionStrategy, Component } from '@angular/core';

/** The application's icon, as in `src-tauri/icons/devnotes.svg`: a dark tile in both themes. */
@Component({
  selector: 'app-logo',
  template: `
    <svg viewBox="0 0 64 64" aria-hidden="true">
      <rect x="4" y="4" width="56" height="56" rx="12" fill="#1b1e25" />
      <rect x="12" y="16" width="28" height="8" rx="4" fill="#8b93a7" />
      <rect x="12" y="28" width="32" height="8" rx="4" fill="#e8a33d" />
      <rect x="48" y="24" width="4" height="16" rx="1" fill="#e8a33d" />
      <rect x="12" y="40" width="20" height="8" rx="4" fill="#8b93a7" />
    </svg>
  `,
  styles: `
    :host {
      display: inline-grid;
      flex: none;
      width: var(--logo-size, 30px);
      height: var(--logo-size, 30px);
    }

    svg {
      width: 100%;
      height: 100%;
    }
  `,
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class Logo {}
