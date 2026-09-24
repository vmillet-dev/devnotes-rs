import { ComponentFixture, TestBed } from '@angular/core/testing';
import { SettingsStore } from '@core/services/settings/settings.store';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { VaultStore } from '@core/state/vault.store';
import { provideAppTesting } from '@testing/testing.providers';
import { TitlebarComponent } from './titlebar.component';

describe('TitlebarComponent', () => {
  let fixture: ComponentFixture<TitlebarComponent>;

  /** With no choice made the active language is the system one, and jsdom's is `en-US`. */
  function stubSystemLanguage(tag: string): void {
    Object.defineProperty(navigator, 'languages', { value: [tag], configurable: true });
    Object.defineProperty(navigator, 'language', { value: tag, configurable: true });
  }

  /** The titlebar is on screen before the library is, so a spec has to say which. */
  async function unlock(): Promise<void> {
    await TestBed.inject(VaultStore).load();
    await fixture.whenStable();
  }

  function menus(): HTMLElement {
    return fixture.nativeElement.querySelector('.titlebar-menus');
  }

  beforeEach(() => {
    TestBed.resetTestingModule();
    localStorage.clear();
    stubSystemLanguage('fr-FR');
    TestBed.configureTestingModule({ imports: [TitlebarComponent], providers: [provideAppTesting()] });
    fixture = TestBed.createComponent(TitlebarComponent);
    fixture.autoDetectChanges();
  });

  afterEach(() => {
    Reflect.deleteProperty(navigator, 'languages');
    Reflect.deleteProperty(navigator, 'language');
  });

  it('renders the application name', () => {
    expect(fixture.nativeElement.querySelector('.titlebar-title').textContent.trim()).toBe('DevNotes');
  });

  /** The window's controls are the native frame's, and the language is the preferences' to choose. */
  it('draws no window controls and no language switch of its own', () => {
    expect(fixture.nativeElement.querySelector('.dot, .dots')).toBeNull();
    expect(fixture.nativeElement.querySelector('[data-testid="locale-option"]')).toBeNull();
  });

  it('groups the menus on the left, ahead of the title', async () => {
    await unlock();
    const title = fixture.nativeElement.querySelector('.titlebar-title');

    expect(menus().querySelector('app-file-menu')).not.toBeNull();
    expect(menus().querySelector('app-about-menu')).not.toBeNull();
    expect(menus().compareDocumentPosition(title)).toBe(Node.DOCUMENT_POSITION_FOLLOWING);
  });

  /** Every File entry acts on the library, and the component itself injects
   *  `SpacesStore`, which would query a database nobody has opened yet. */
  it('keeps the File menu out of the titlebar until the library is open', async () => {
    expect(menus().querySelector('app-file-menu')).toBeNull();
    expect(menus().querySelector('app-about-menu')).not.toBeNull();

    await unlock();

    expect(menus().querySelector('app-file-menu')).not.toBeNull();
  });

  /**
   * Light or dark, never "system": a click is expected to change what is on screen, and
   * "system" usually looks exactly like what was already there. The panel keeps the three.
   */
  describe('the theme', () => {
    function control(): HTMLButtonElement {
      return fixture.nativeElement.querySelector('[data-testid="theme-toggle"]');
    }

    async function press(): Promise<void> {
      control().click();
      await fixture.whenStable();
    }

    it('is one control, showing the theme on screen', async () => {
      TestBed.inject(SettingsStore).setTheme('dark');
      await fixture.whenStable();

      expect(fixture.nativeElement.querySelectorAll('[data-testid="theme-toggle"]')).toHaveLength(1);
      expect(control().getAttribute('data-theme-shown')).toBe('dark');
    });

    it('toggles between light and dark', async () => {
      const settings = TestBed.inject(SettingsStore);
      settings.setTheme('light');
      await fixture.whenStable();

      const walked: string[] = [];
      for (let step = 0; step < 3; step += 1) {
        await press();
        walked.push(settings.theme());
      }

      expect(walked).toEqual(['dark', 'light', 'dark']);
    });

    /** jsdom's system resolves to light, so the opposite of the screen is dark. */
    it('leaves "system" for the explicit opposite of what it resolved to', async () => {
      const settings = TestBed.inject(SettingsStore);
      settings.setTheme('system');
      await fixture.whenStable();
      expect(control().getAttribute('data-theme-shown')).toBe('light');

      await press();

      expect(settings.theme()).toBe('dark');
    });

    /** The icon is decorative: the label says what a press will do. */
    it('names the action in words, which the icon does not', async () => {
      TestBed.inject(SettingsStore).setTheme('dark');
      await fixture.whenStable();

      expect(control().getAttribute('aria-label')).toContain('clair');
      expect(control().querySelector('svg')?.getAttribute('aria-hidden')).toBe('true');
    });
  });
});
