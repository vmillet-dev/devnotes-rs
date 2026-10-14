import { DOCUMENT, Injectable, inject, signal } from '@angular/core';

export type Theme = 'light' | 'dark';

/** Read before the first paint by the inline script in index.html: keep both in step. */
const STORAGE_KEY = 'devnotes-site-theme';

function stored(): Theme | null {
  try {
    const value = localStorage.getItem(STORAGE_KEY);
    return value === 'light' || value === 'dark' ? value : null;
  } catch {
    return null;
  }
}

/**
 * The visitor's system decides until the toggle is used; from then on their choice wins,
 * kept in `localStorage` when the browser allows it. The page is right without it.
 */
@Injectable({ providedIn: 'root' })
export class ThemeStore {
  private readonly document = inject(DOCUMENT);
  private readonly _theme = signal<Theme>('dark');
  readonly theme = this._theme.asReadonly();

  /** Browser only: adopts what the inline script applied, then follows the system until told otherwise. */
  connect(): void {
    this._theme.set(this.document.documentElement.dataset['theme'] === 'light' ? 'light' : 'dark');
    if (typeof matchMedia !== 'function') return;

    matchMedia('(prefers-color-scheme: light)').addEventListener('change', (event) => {
      if (stored() === null) this.apply(event.matches ? 'light' : 'dark');
    });
  }

  toggle(): void {
    const next: Theme = this._theme() === 'dark' ? 'light' : 'dark';
    try {
      localStorage.setItem(STORAGE_KEY, next);
    } catch {
      // A private window: the choice lasts as long as the page.
    }
    this.apply(next);
  }

  private apply(theme: Theme): void {
    this.document.documentElement.dataset['theme'] = theme;
    this._theme.set(theme);
  }
}
