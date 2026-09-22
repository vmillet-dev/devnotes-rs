import { ComponentFixture, TestBed } from '@angular/core/testing';
import { By } from '@angular/platform-browser';
import { beforeEach, describe, expect, it } from 'vitest';
import { SettingsStore } from '@core/services/settings/settings.store';
import { UpdateStore } from '@core/services/updates/update.store';
import { UpdaterService } from '@core/services/updates/updater.service';
import { FakeUpdater } from '@testing/fake-updater';
import { provideTranslocoTesting } from '@testing/provide-transloco-testing';
import { UpdatePromptComponent } from './update-prompt.component';

describe('UpdatePromptComponent', () => {
  let fixture: ComponentFixture<UpdatePromptComponent>;
  let updater: FakeUpdater;
  let store: UpdateStore;

  /** Puts an update on the table, as the startup check would. */
  async function offerUpdate(notes?: string): Promise<void> {
    updater.available = { version: '0.2.0', currentVersion: '0.1.0', notes };
    await store.check();
    await fixture.whenStable();
  }

  beforeEach(() => {
    TestBed.resetTestingModule();
    updater = new FakeUpdater();
    TestBed.configureTestingModule({
      imports: [UpdatePromptComponent],
      providers: [{ provide: UpdaterService, useValue: updater }, provideTranslocoTesting()],
    });
    store = TestBed.inject(UpdateStore);
    fixture = TestBed.createComponent(UpdatePromptComponent);
    fixture.autoDetectChanges();
  });

  it('renders nothing while no update is available', () => {
    expect(fixture.debugElement.query(By.css('.dialog-panel'))).toBeNull();
  });

  it('announces the available version and the one in use', async () => {
    await offerUpdate();

    const summary = fixture.nativeElement.querySelector('.update-summary').textContent;
    expect(summary).toContain('0.2.0');
    expect(summary).toContain('0.1.0');
  });

  it('names itself to the dialog shell by its own title', async () => {
    await offerUpdate();

    const panel = fixture.nativeElement.querySelector('.dialog-panel');

    expect(panel.getAttribute('aria-labelledby')).toBe('update-prompt-title');
    expect(fixture.nativeElement.querySelector('#update-prompt-title')).not.toBeNull();
  });

  it('shows the release notes only when the manifest carries some', async () => {
    await offerUpdate();
    expect(fixture.debugElement.query(By.css('.update-notes'))).toBeNull();

    await store.dismiss();
    await offerUpdate('Corrige le rail de tags');
    expect(fixture.nativeElement.querySelector('.update-notes-body').textContent).toContain(
      'Corrige le rail de tags',
    );
  });

  /**
   * ⚠️ Closed by default meant installing without reading — and worse, opening it changed
   * the dialog's height: the body is bounded but the collapsed summary is one line, so the
   * panel grew under the cursor and moved "Install now" out from under the mouse.
   */
  it('shows them without a click, and behind nothing that can be folded', async () => {
    await offerUpdate('Corrige le rail de tags');

    expect(fixture.debugElement.query(By.css('.update-notes details'))).toBeNull();
    expect(fixture.debugElement.query(By.css('.update-notes summary'))).toBeNull();
    const body = fixture.nativeElement.querySelector('.update-notes-body') as HTMLElement;
    expect(body.closest('details')).toBeNull();
  });

  describe('the skip checkbox', () => {
    const box = (): HTMLInputElement => fixture.nativeElement.querySelector('[data-testid="update-skip"]');

    it('writes the version down, never a bare "no"', async () => {
      await offerUpdate();

      box().click();
      await fixture.whenStable();
      fixture.debugElement.query(By.css('[data-testid="update-later"]')).triggerEventHandler('click');
      await fixture.whenStable();

      expect(TestBed.inject(SettingsStore).skippedUpdate()).toBe('0.2.0');
    });

    it('leaves the version alone when the box is untouched', async () => {
      await offerUpdate();

      fixture.debugElement.query(By.css('[data-testid="update-later"]')).triggerEventHandler('click');
      await fixture.whenStable();

      expect(TestBed.inject(SettingsStore).skippedUpdate()).toBe('');
    });

    /** Escape and the backdrop produce no click on "Later", and must honour it too. */
    it('is honoured by the ways out that are not the button', async () => {
      await offerUpdate();

      box().click();
      await fixture.whenStable();
      document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }));
      await fixture.whenStable();

      expect(TestBed.inject(SettingsStore).skippedUpdate()).toBe('0.2.0');
    });
  });

  it('installs nothing until the user says so', async () => {
    await offerUpdate();

    expect(updater.installCalls).toBe(0);
  });

  it('starts the install on the primary button', async () => {
    updater.deferInstall = true;
    await offerUpdate();

    fixture.debugElement.query(By.css('.update-btn.primary')).triggerEventHandler('click');
    await fixture.whenStable();

    expect(updater.installCalls).toBe(1);
    expect(fixture.debugElement.query(By.css('.update-actions'))).toBeNull();
    expect(fixture.nativeElement.querySelector('.update-status')).not.toBeNull();

    updater.finishInstall();
  });

  it('leaves the progress bar indeterminate while the size is unknown', async () => {
    updater.deferInstall = true;
    updater.progressSteps = [null];
    await offerUpdate();

    fixture.debugElement.query(By.css('.update-btn.primary')).triggerEventHandler('click');
    await fixture.whenStable();

    expect(fixture.nativeElement.querySelector('.update-progress').hasAttribute('value')).toBe(false);

    updater.finishInstall();
  });

  it('reflects the download percentage on the progress bar', async () => {
    updater.deferInstall = true;
    updater.progressSteps = [0.6];
    await offerUpdate();

    fixture.debugElement.query(By.css('.update-btn.primary')).triggerEventHandler('click');
    await fixture.whenStable();

    expect(fixture.nativeElement.querySelector('.update-progress').getAttribute('value')).toBe('60');

    updater.finishInstall();
  });

  it('closes on the secondary button', async () => {
    await offerUpdate();

    fixture.debugElement.query(By.css('.update-btn:not(.primary)')).triggerEventHandler('click');
    await fixture.whenStable();

    expect(fixture.debugElement.query(By.css('.dialog-panel'))).toBeNull();
    expect(updater.discarded).toBe(true);
  });

  it('closes on Escape', async () => {
    await offerUpdate();

    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }));
    await fixture.whenStable();

    expect(fixture.debugElement.query(By.css('.dialog-panel'))).toBeNull();
  });

  it('ignores Escape once the install is under way', async () => {
    updater.deferInstall = true;
    await offerUpdate();
    fixture.debugElement.query(By.css('.update-btn.primary')).triggerEventHandler('click');
    await fixture.whenStable();

    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }));
    await fixture.whenStable();

    expect(fixture.debugElement.query(By.css('.dialog-panel'))).not.toBeNull();

    updater.finishInstall();
  });
});
