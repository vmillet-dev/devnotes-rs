import { ComponentFixture, TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { SettingsDraftStore } from '@core/services/settings/settings-draft.store';
import { SettingsStore } from '@core/services/settings/settings.store';
import { VariablesStore } from '@core/state/variables.store';
import { provideAppTesting } from '@testing/testing.providers';
import { SettingsDialogComponent } from './settings-dialog.component';

describe('SettingsDialogComponent', () => {
  let fixture: ComponentFixture<SettingsDialogComponent>;

  function railOptions(): HTMLButtonElement[] {
    return [...fixture.nativeElement.querySelectorAll('.settings-rail-option')];
  }

  function optionLabelled(label: string): HTMLButtonElement {
    const found = railOptions().find((option) => option.textContent?.includes(label));
    if (!found) throw new Error(`No rail option labelled "${label}"`);
    return found;
  }

  async function render(): Promise<void> {
    fixture = TestBed.createComponent(SettingsDialogComponent);
    fixture.autoDetectChanges();
    await fixture.whenStable();
  }

  beforeEach(() => {
    TestBed.resetTestingModule();
    TestBed.configureTestingModule({
      imports: [SettingsDialogComponent],
      providers: [provideAppTesting()],
    });
  });

  it('names itself to the dialog shell by its own title', async () => {
    await render();

    const panel = fixture.nativeElement.querySelector('.dialog-panel');

    expect(panel.getAttribute('aria-labelledby')).toBe('settings-dialog-title');
  });

  it('lists its pages in the order it declares them', async () => {
    await render();

    expect(railOptions().map((option) => option.textContent?.trim())).toEqual([
      'Général',
      'Raccourcis',
      'Sécurité',
      'Variables',
    ]);
  });

  it('opens on the general page, which is what the menu entry promised', async () => {
    await render();

    expect(optionLabelled('Général').getAttribute('aria-current')).toBe('page');
    expect(fixture.nativeElement.querySelector('app-variables-page')).toBeNull();
  });

  it('renders the chosen page through the outlet', async () => {
    await render();

    optionLabelled('Variables').click();
    await fixture.whenStable();

    expect(optionLabelled('Variables').getAttribute('aria-current')).toBe('page');
    expect(fixture.nativeElement.querySelector('app-variables-page')).not.toBeNull();
  });

  function button(testid: string): HTMLButtonElement {
    return fixture.nativeElement.querySelector(`[data-testid="${testid}"]`);
  }

  it('emits on OK, with nothing waiting', async () => {
    await render();
    const closed = vi.fn();
    fixture.componentInstance.closed.subscribe(closed);

    button('settings-close').click();
    await fixture.whenStable();

    expect(closed).toHaveBeenCalledTimes(1);
  });

  /**
   * The variables are corpus data with a store of their own, but they are edited under
   * the same footer: a panel where one page committed on blur and three waited for a
   * button would be worse than either rule on its own.
   */
  describe('the variables page, under the same footer', () => {
    let variables: VariablesStore;

    beforeEach(async () => {
      variables = TestBed.inject(VariablesStore);
      await render();
    });

    it('counts an edit as something waiting', async () => {
      expect(button('settings-apply').disabled).toBe(true);

      variables.add();
      await fixture.whenStable();

      expect(button('settings-apply').disabled).toBe(false);
      expect(button('settings-unapplied')).toBeNull();
    });

    it('holds the panel open rather than dropping the rows on a backdrop click', async () => {
      variables.add();
      await fixture.whenStable();

      fixture.nativeElement.querySelector('.dialog-backdrop').click();
      await fixture.whenStable();

      expect(button('settings-unapplied')).not.toBeNull();
      expect(variables.variables()).toHaveLength(1);
    });
  });

  describe('a draft waiting to be applied', () => {
    let draft: SettingsDraftStore;
    let settings: SettingsStore;

    beforeEach(async () => {
      draft = TestBed.inject(SettingsDraftStore);
      settings = TestBed.inject(SettingsStore);
      draft.cancel();
      await render();
      draft.set('density', 'compact');
      await fixture.whenStable();
    });

    it('offers Appliquer only once there is something to apply', async () => {
      draft.cancel();
      await fixture.whenStable();
      expect(button('settings-apply').disabled).toBe(true);

      draft.set('density', 'compact');
      await fixture.whenStable();

      expect(button('settings-apply').disabled).toBe(false);
    });

    it('writes on Appliquer and keeps the panel open', async () => {
      const closed = vi.fn();
      fixture.componentInstance.closed.subscribe(closed);

      button('settings-apply').click();
      await fixture.whenStable();

      expect(settings.density()).toBe('compact');
      expect(closed).not.toHaveBeenCalled();
      expect(draft.isDirty()).toBe(false);
    });

    it('writes and closes on OK, which is the button that was aimed at it', async () => {
      const closed = vi.fn();
      fixture.componentInstance.closed.subscribe(closed);

      button('settings-close').click();
      await fixture.whenStable();

      expect(closed).toHaveBeenCalledTimes(1);
      expect(settings.density()).toBe('compact');
    });

    /**
     * Escape and the backdrop produce no click, so without this guard the two of them
     * are a silent Annuler — the one outcome nobody would have chosen on purpose.
     */
    it('holds the panel open when the shell asks to close with work in hand', async () => {
      const closed = vi.fn();
      fixture.componentInstance.closed.subscribe(closed);

      fixture.nativeElement.querySelector('.dialog-backdrop').click();
      await fixture.whenStable();

      expect(closed).not.toHaveBeenCalled();
      expect(button('settings-unapplied')).not.toBeNull();
      expect(settings.density()).toBe('comfortable');
    });

    it('drops the draft on "close without applying"', async () => {
      fixture.nativeElement.querySelector('.dialog-backdrop').click();
      await fixture.whenStable();
      const closed = vi.fn();
      fixture.componentInstance.closed.subscribe(closed);

      button('settings-discard').click();
      await fixture.whenStable();

      expect(settings.density()).toBe('comfortable');
      expect(draft.isDirty()).toBe(false);
      expect(closed).toHaveBeenCalledTimes(1);
    });

    it('writes it on "apply and close"', async () => {
      fixture.nativeElement.querySelector('.dialog-backdrop').click();
      await fixture.whenStable();
      const closed = vi.fn();
      fixture.componentInstance.closed.subscribe(closed);

      button('settings-apply-close').click();
      await fixture.whenStable();

      expect(settings.density()).toBe('compact');
      expect(closed).toHaveBeenCalledTimes(1);
    });
  });
});
