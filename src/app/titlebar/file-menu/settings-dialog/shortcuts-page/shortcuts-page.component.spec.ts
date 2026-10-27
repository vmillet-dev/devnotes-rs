import { ComponentFixture, TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { GlobalShortcutsService } from '@core/services/shortcuts/global-shortcuts.service';
import { PreferencesService } from '@core/services/preferences/preferences.service';
import { SettingsDraftStore } from '@core/services/settings/settings-draft.store';
import { SettingsStore } from '@core/services/settings/settings.store';
import { provideTranslocoTesting } from '@testing/provide-transloco-testing';
import { ShortcutsPageComponent } from './shortcuts-page.component';

describe('ShortcutsPageComponent', () => {
  let fixture: ComponentFixture<ShortcutsPageComponent>;
  let settings: SettingsStore;
  let preferences: PreferencesService;
  let draft: SettingsDraftStore;

  const field = (id: string): HTMLInputElement =>
    fixture.nativeElement.querySelector(`[data-testid="shortcut-field"][data-action="${id}"]`);

  const reset = (id: string): HTMLButtonElement =>
    fixture.nativeElement.querySelector(`[data-testid="shortcut-reset"][data-action="${id}"]`);

  const error = (id: string): string | null =>
    fixture.nativeElement.querySelector(`[id="shortcut-error-${id}"]`)?.textContent?.trim() ?? null;

  function press(id: string, init: KeyboardEventInit): KeyboardEvent {
    const event = new KeyboardEvent('keydown', { ...init, bubbles: true, cancelable: true });
    field(id).dispatchEvent(event);
    return event;
  }

  beforeEach(async () => {
    TestBed.resetTestingModule();
    TestBed.configureTestingModule({
      imports: [ShortcutsPageComponent],
      providers: [provideTranslocoTesting()],
    });
    settings = TestBed.inject(SettingsStore);
    preferences = TestBed.inject(PreferencesService);
    draft = TestBed.inject(SettingsDraftStore);
    draft.cancel();
    fixture = TestBed.createComponent(ShortcutsPageComponent);
    fixture.autoDetectChanges();
    await fixture.whenStable();
  });

  /** Every rebindable action, the canvas ones included, not only the three global ones. */
  it('offers every rebindable action, the canvas ones included', () => {
    const fields = [...fixture.nativeElement.querySelectorAll('[data-testid="shortcut-field"]')];

    expect(fields.length).toBeGreaterThan(3);
    expect(field('palette').value).toBe('Ctrl+Alt+P');
    expect(field('canvas.copy').value).toBe('C');
  });

  /** Staged, not registered: a half-captured global shortcut must not be live machine-wide. */
  it('records a global shortcut from the keystroke, and holds it until it is applied', async () => {
    press('palette', { code: 'KeyK', ctrlKey: true, shiftKey: true });
    await fixture.whenStable();

    expect(field('palette').value).toBe('Ctrl+Shift+K');
    expect(settings.paletteShortcut()).toBe('Ctrl+Alt+P');

    draft.apply();

    expect(settings.paletteShortcut()).toBe('Ctrl+Shift+K');
  });

  /**
   * The two halves read the keyboard differently on purpose: a global key is stored by
   * position, because the native parser reads it back that way, and a canvas key by the
   * printed character — `C` is read off the keycap.
   */
  it('stores a canvas key by the letter on the cap, not by the key position', async () => {
    press('canvas.copy', { key: 'y', code: 'KeyZ' });
    await fixture.whenStable();

    expect(field('canvas.copy').value).toBe('Y');
    expect(preferences.read('devnotes.shortcut.canvas.copy')).toBeNull();

    draft.apply();

    expect(preferences.read('devnotes.shortcut.canvas.copy')).toBe('Y');
  });

  /** A canvas key answers only while the canvas has the keyboard, so a bare one is fine. */
  it('takes a bare key on the canvas half', async () => {
    press('canvas.pin', { key: 'j' });
    await fixture.whenStable();

    expect(field('canvas.pin').value).toBe('J');
    expect(error('canvas.pin')).toBeNull();
  });

  /** A global key with no modifier would swallow that key in every application. */
  it('refuses a bare key on the global half, and says why', async () => {
    press('palette', { key: 'j', code: 'KeyJ' });
    await fixture.whenStable();

    expect(field('palette').value).toBe('Ctrl+Alt+P');
    expect(error('palette')).toContain('Ctrl, Alt, Maj');
  });

  it('lets Tab through, which is what keeps the field escapable', async () => {
    const tab = press('palette', { key: 'Tab', code: 'Tab' });
    await fixture.whenStable();

    expect(tab.defaultPrevented).toBe(false);
    expect(error('palette')).toBeNull();
  });

  it('refuses a key the canvas keeps for itself, and says so', async () => {
    press('canvas.pin', { key: 'ArrowLeft', code: 'ArrowLeft' });
    await fixture.whenStable();

    expect(field('canvas.pin').value).toBe('P');
    expect(error('canvas.pin')).toBe('Cette touche est réservée à la navigation.');
  });

  it('says nothing of a modifier alone, a combination still being typed', async () => {
    press('canvas.pin', { key: 'Shift', code: 'ShiftLeft', shiftKey: true });
    await fixture.whenStable();

    expect(error('canvas.pin')).toBeNull();
  });

  it('lets go of the global shortcuts while a global field listens, and only then', async () => {
    const shortcuts = TestBed.inject(GlobalShortcutsService);
    const suspend = vi.spyOn(shortcuts, 'suspend');
    const resume = vi.spyOn(shortcuts, 'resume');

    field('canvas.pin').dispatchEvent(new Event('focus'));
    expect(suspend).not.toHaveBeenCalled();

    field('palette').dispatchEvent(new Event('focus'));
    expect(suspend).toHaveBeenCalledTimes(1);

    field('palette').dispatchEvent(new Event('blur'));
    expect(resume).toHaveBeenCalledTimes(1);

    fixture.destroy();
    expect(resume).toHaveBeenCalledTimes(2);
  });

  it('refuses a global combination another global shortcut holds, naming it', async () => {
    press('palette', { key: 'n', code: 'KeyN', ctrlKey: true, altKey: true });
    await fixture.whenStable();

    expect(field('palette').value).toBe('Ctrl+Alt+P');
    expect(error('palette')).toContain('Ctrl+Alt+N');
    expect(error('palette')).toContain('Créer une note');
  });

  /** The dialog listens on the document: the first Escape stops at the field, the second reaches it. */
  it('stops listening on a first Escape, and lets the second one close', async () => {
    const first = press('canvas.pin', { key: 'Escape', code: 'Escape' });
    await fixture.whenStable();
    expect(first.defaultPrevented).toBe(true);
    expect(error('canvas.pin')).toBeNull();

    const second = press('canvas.pin', { key: 'Escape', code: 'Escape' });
    expect(second.defaultPrevented).toBe(false);

    press('canvas.pin', { key: 'j' });
    await fixture.whenStable();
    expect(field('canvas.pin').value).toBe('P');

    field('canvas.pin').dispatchEvent(new Event('focus'));
    press('canvas.pin', { key: 'j' });
    await fixture.whenStable();
    expect(field('canvas.pin').value).toBe('J');
  });

  /** The second action would be unreachable, and nothing on screen would say which. */
  it('refuses a keystroke another action already answers to, naming it', async () => {
    press('canvas.pin', { key: 'c' });
    await fixture.whenStable();

    expect(field('canvas.pin').value).toBe('P');
    expect(error('canvas.pin')).toContain('C');
    expect(error('canvas.pin')).toContain('Copier');
  });

  /** Across the two storage paths, or a key could be taken twice between them. */
  it('sees a collision between the two halves, not only inside one', async () => {
    press('palette', { code: 'KeyB', ctrlKey: true });
    await fixture.whenStable();

    expect(field('palette').value).toBe('Ctrl+Alt+P');
    expect(error('palette')).toContain('Ctrl+B');
  });

  it('offers to restore a default, and only once it has moved', async () => {
    expect(reset('canvas.copy').disabled).toBe(true);

    press('canvas.copy', { key: 'y' });
    await fixture.whenStable();
    expect(reset('canvas.copy').disabled).toBe(false);

    reset('canvas.copy').click();
    await fixture.whenStable();
    draft.apply();

    expect(field('canvas.copy').value).toBe('C');
    expect(preferences.read('devnotes.shortcut.canvas.copy')).toBeNull();
  });

  /**
   * The fields refuse a collision, so this answers for what they cannot refuse: a
   * preferences file written by hand, or a shipped default landing on a taken key.
   */
  it('reports a collision it could not have refused', async () => {
    preferences.write('devnotes.shortcut.canvas.pin', 'C');

    // A second component, so the computed is built once the value is already there:
    // nothing bumped the revision, because nothing went through `rebind`.
    const reopened = TestBed.createComponent(ShortcutsPageComponent);
    reopened.autoDetectChanges();
    await reopened.whenStable();

    const conflict = reopened.nativeElement.querySelector('[data-testid="shortcut-conflict"]');
    expect(conflict?.textContent).toContain('Copier');
  });
});
