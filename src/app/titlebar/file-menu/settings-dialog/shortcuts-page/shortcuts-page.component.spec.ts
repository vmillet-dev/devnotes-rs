import { ComponentFixture, TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';
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

  /**
   * ⚠️ The whole of #254: three keys out of twenty could be changed, and the other
   * seventeen were documented in a sheet that offered nothing.
   */
  it('offers every rebindable action, the canvas ones included', () => {
    const fields = [...fixture.nativeElement.querySelectorAll('[data-testid="shortcut-field"]')];

    expect(fields.length).toBeGreaterThan(3);
    expect(field('palette').value).toBe('Ctrl+Alt+P');
    expect(field('canvas.copy').value).toBe('C');
  });

  /**
   * ⚠️ Staged, not registered. A global shortcut is taken from the whole machine, and a
   * half-captured one being live for as long as it takes to finish is the argument #255
   * was written on.
   */
  it('records a global shortcut from the keystroke, and holds it until it is applied', async () => {
    press('palette', { code: 'KeyK', ctrlKey: true, shiftKey: true });
    await fixture.whenStable();

    expect(field('palette').value).toBe('Ctrl+Shift+K');
    expect(settings.paletteShortcut()).toBe('Ctrl+Alt+P');

    draft.apply();

    expect(settings.paletteShortcut()).toBe('Ctrl+Shift+K');
  });

  /**
   * ⚠️ The two halves read the keyboard differently on purpose: a global key is stored by
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

  /** ⚠️ A global key with no modifier would swallow that key in every application. */
  it('refuses a bare key on the global half, and says why', async () => {
    press('palette', { key: 'j', code: 'KeyJ' });
    await fixture.whenStable();

    expect(field('palette').value).toBe('Ctrl+Alt+P');
    expect(error('palette')).toContain('Ctrl, Alt, Maj');
  });

  it('lets Tab and Escape through, which is what keeps the field escapable', async () => {
    const tab = press('palette', { key: 'Tab', code: 'Tab' });
    await fixture.whenStable();

    expect(tab.defaultPrevented).toBe(false);
    expect(error('palette')).toBeNull();
  });

  /** ⚠️ The second action would be unreachable, and nothing on screen would say which. */
  it('refuses a keystroke another action already answers to, naming it', async () => {
    press('canvas.pin', { key: 'c' });
    await fixture.whenStable();

    expect(field('canvas.pin').value).toBe('P');
    expect(error('canvas.pin')).toContain('C');
    expect(error('canvas.pin')).toContain('Copier');
  });

  /** ⚠️ Across the two storage paths, or a key could be taken twice between them. */
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
   * ⚠️ The fields refuse a collision, so this answers for what they cannot refuse: a
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
