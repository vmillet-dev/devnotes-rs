import { ComponentFixture, TestBed } from '@angular/core/testing';
import { By } from '@angular/platform-browser';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { ErrorBannerComponent } from '@banners/error-banner/error-banner.component';
import { NotesPageComponent } from '@notes/notes-page.component';
import { TitlebarComponent } from '@titlebar/titlebar.component';
import { UpdatePromptComponent } from '@banners/update-prompt/update-prompt.component';
import { provideAppTesting } from '@testing/testing.providers';
import { VaultGateComponent } from './vault-gate/vault-gate.component';
import { VaultStore } from '@core/state/vault.store';
import { VaultRepository } from '@core/data/vault.repository';
import { FakeVaultRepository } from '@testing/fake-vault-repository';
import { RAIL_ACTION } from '@core/services/areas/area.model';
import { AreaStore } from '@core/services/areas/area.store';
import { SettingsStore } from '@core/services/settings/settings.store';
import { ShortcutBindingsStore } from '@core/services/shortcuts/shortcut-bindings.store';
import { ToolsStore } from '@core/services/tools/tools.store';
import { ToolsHomeComponent } from '@tools/home/tools-home.component';
import { ToolsPageComponent } from '@tools/tools-page.component';
import { AppComponent } from './app.component';

describe('AppComponent', () => {
  let fixture: ComponentFixture<AppComponent>;

  beforeEach(() => {
    TestBed.resetTestingModule();
    localStorage.clear();
    TestBed.configureTestingModule({
      imports: [AppComponent],
      providers: [provideAppTesting()],
    });
    fixture = TestBed.createComponent(AppComponent);
    fixture.autoDetectChanges();
  });

  /** The shell asks Rust before the first render; a spec says what the answer was. */
  async function withVault(state: 'absent' | 'locked' | 'unlocked'): Promise<void> {
    (TestBed.inject(VaultRepository) as unknown as FakeVaultRepository).answer = state;
    await TestBed.inject(VaultStore).load();
    await fixture.whenStable();
  }

  it('renders the persistent chrome: titlebar, global error banner and update prompt', () => {
    expect(fixture.debugElement.query(By.directive(TitlebarComponent))).not.toBeNull();
    expect(fixture.debugElement.query(By.directive(ErrorBannerComponent))).not.toBeNull();
    expect(fixture.debugElement.query(By.directive(UpdatePromptComponent))).not.toBeNull();
  });

  it('shows the notes page once the library is unlocked', async () => {
    await withVault('unlocked');

    // Deferred: its chunk can land after the first stable render.
    await vi.waitFor(() =>
      expect(fixture.debugElement.query(By.directive(NotesPageComponent))).not.toBeNull(),
    );
    expect(fixture.debugElement.query(By.directive(VaultGateComponent))).toBeNull();
  });

  /**
   * The page is not merely hidden while the library is locked: it is not created.
   * The canvas queries notes the moment it mounts, and there would be nothing to answer.
   */
  it('puts the gate in front of the notes page while the library is locked', async () => {
    await withVault('locked');

    expect(fixture.debugElement.query(By.directive(VaultGateComponent))).not.toBeNull();
    expect(fixture.debugElement.query(By.directive(NotesPageComponent))).toBeNull();
  });

  /** A library that has never been encrypted asks for a passphrase to be chosen. */
  it('asks for a passphrase to be created on a library that has none', async () => {
    await withVault('absent');

    expect(fixture.debugElement.query(By.directive(VaultGateComponent))).not.toBeNull();
  });

  /** Before the answer lands, neither: the shell would flash a screen it is replacing. */
  it('renders neither until Rust has answered', () => {
    expect(fixture.debugElement.query(By.directive(VaultGateComponent))).toBeNull();
    expect(fixture.debugElement.query(By.directive(NotesPageComponent))).toBeNull();
  });

  describe('the areas', () => {
    const press = (code: string) =>
      document.dispatchEvent(new KeyboardEvent('keydown', { code, key: '?', ctrlKey: true, bubbles: true }));
    const notesPage = () => fixture.debugElement.query(By.directive(NotesPageComponent));

    it('opens the tools on Ctrl+2, and keeps the notes behind them', async () => {
      await withVault('unlocked');

      press('Digit2');
      await fixture.whenStable();

      expect(TestBed.inject(AreaStore).current()).toBe('tools');
      expect(fixture.debugElement.query(By.directive(ToolsPageComponent))).not.toBeNull();
      expect(notesPage().nativeElement.classList).toContain('away');

      press('Digit1');
      await fixture.whenStable();

      expect(fixture.debugElement.query(By.directive(ToolsPageComponent))).toBeNull();
      expect(notesPage().nativeElement.classList).not.toContain('away');
    });

    /**
     * One state, one key, three areas: the canvas had the key, HTTP a copy of it, the tools
     * neither — and they drew their rail whatever had been chosen.
     */
    it('hides and shows the rail on Ctrl+B in the three areas alike', async () => {
      await withVault('unlocked');
      const settings = TestBed.inject(SettingsStore);
      const ctrlB = (target: EventTarget = document) =>
        target.dispatchEvent(
          new KeyboardEvent('keydown', {
            key: 'b',
            code: 'KeyB',
            ctrlKey: true,
            bubbles: true,
            cancelable: true,
          }),
        );

      for (const [code, shown] of [
        ['Digit1', false],
        ['Digit2', true],
        ['Digit3', false],
      ] as const) {
        press(code);
        await fixture.whenStable();
        ctrlB();
        await fixture.whenStable();

        expect(settings.showLibraryRail()).toBe(shown);
      }

      press('Digit2');
      await fixture.whenStable();
      expect(fixture.nativeElement.querySelector('[data-testid="tools-rail"]')).toBeNull();
      expect(fixture.nativeElement.querySelector('[data-testid="tools-rail-toggle"]')).not.toBeNull();
    });

    /** From a field too, where the key types nothing — but not where it means bold. */
    it('answers Ctrl+B from a field, and leaves it to an editor that formats with it', async () => {
      await withVault('unlocked');
      const settings = TestBed.inject(SettingsStore);
      const pressIn = (target: HTMLElement) => {
        document.body.appendChild(target);
        const event = new KeyboardEvent('keydown', {
          key: 'b',
          code: 'KeyB',
          ctrlKey: true,
          bubbles: true,
          cancelable: true,
        });
        target.dispatchEvent(event);
        target.remove();
        return event.defaultPrevented;
      };

      const field = document.createElement('input');
      expect(pressIn(field)).toBe(true);
      expect(settings.showLibraryRail()).toBe(false);

      const editor = document.createElement('div');
      // jsdom computes no `isContentEditable`: the editor's own answer is given to it.
      Object.defineProperty(editor, 'isContentEditable', { value: true });
      expect(pressIn(editor)).toBe(false);
      expect(settings.showLibraryRail()).toBe(false);
    });

    /** Moved onto a bare letter, it must not fire while that letter is being typed. */
    it('follows the key the rail is bound to, and never takes a bare letter from a field', async () => {
      await withVault('unlocked');
      const settings = TestBed.inject(SettingsStore);
      TestBed.inject(ShortcutBindingsStore).store(RAIL_ACTION, 'L');
      const letter = (target: EventTarget) =>
        target.dispatchEvent(
          new KeyboardEvent('keydown', { key: 'l', code: 'KeyL', bubbles: true, cancelable: true }),
        );

      const field = document.createElement('input');
      document.body.appendChild(field);
      letter(field);
      field.remove();
      expect(settings.showLibraryRail()).toBe(true);

      letter(document);
      expect(settings.showLibraryRail()).toBe(false);

      document.dispatchEvent(
        new KeyboardEvent('keydown', {
          key: 'b',
          code: 'KeyB',
          ctrlKey: true,
          bubbles: true,
          cancelable: true,
        }),
      );
      expect(settings.showLibraryRail()).toBe(false);
    });

    it('opens the tools on their search from the notes, on Ctrl+Shift+T', async () => {
      await withVault('unlocked');

      document.dispatchEvent(
        new KeyboardEvent('keydown', {
          key: 'T',
          code: 'KeyT',
          ctrlKey: true,
          shiftKey: true,
          bubbles: true,
        }),
      );
      await fixture.whenStable();

      expect(TestBed.inject(AreaStore).current()).toBe('tools');
      expect(fixture.debugElement.query(By.directive(ToolsHomeComponent))).not.toBeNull();
      // The home took the request: its field has the focus, where a document holds it.
      expect(TestBed.inject(ToolsStore).searchWanted()).toBe(false);
    });

    it('answers no key behind the gate', async () => {
      await withVault('locked');

      press('Digit2');

      expect(TestBed.inject(AreaStore).current()).toBe('notes');
    });
  });
});
