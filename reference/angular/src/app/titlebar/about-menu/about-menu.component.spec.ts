import { ComponentFixture, TestBed } from '@angular/core/testing';
import { By } from '@angular/platform-browser';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { AppInfoService } from '@core/services/app-info/app-info.service';
import { ChangelogService } from '@core/services/app-info/changelog.service';
import { UpdateStore } from '@core/services/updates/update.store';
import { UpdaterService } from '@core/services/updates/updater.service';
import { AboutDialogComponent } from '@titlebar/about-menu/about-dialog/about-dialog.component';
import { GettingStartedDialogComponent } from '@titlebar/about-menu/getting-started-dialog/getting-started-dialog.component';
import { ShortcutsDialogComponent } from '@titlebar/about-menu/shortcuts-dialog/shortcuts-dialog.component';
import { WhatsNewDialogComponent } from '@titlebar/about-menu/whats-new-dialog/whats-new-dialog.component';
import { FakeAppInfo } from '@testing/fake-app-info';
import { FakeChangelog } from '@testing/fake-changelog';
import { FakeUpdater } from '@testing/fake-updater';
import { provideTranslocoTesting } from '@testing/provide-transloco-testing';
import { AboutMenuComponent } from './about-menu.component';

describe('AboutMenuComponent', () => {
  let fixture: ComponentFixture<AboutMenuComponent>;
  let updater: FakeUpdater;
  let store: UpdateStore;

  const trigger = (): HTMLButtonElement => fixture.nativeElement.querySelector('.about-trigger');
  const options = (): HTMLButtonElement[] =>
    Array.from(fixture.nativeElement.querySelectorAll('.about-option'));

  /** The entries are named, never numbered: adding one must not renumber a spec. */
  function option(label: string): HTMLButtonElement {
    const found = options().find((entry) => entry.textContent?.includes(label));
    if (!found) throw new Error(`No menu entry labelled "${label}"`);

    return found;
  }

  async function openMenu(): Promise<void> {
    trigger().click();
    await fixture.whenStable();
  }

  async function openPanel(label: string): Promise<void> {
    await openMenu();
    option(label).click();
    await fixture.whenStable();
  }

  beforeEach(() => {
    TestBed.resetTestingModule();
    updater = new FakeUpdater();
    TestBed.configureTestingModule({
      imports: [AboutMenuComponent],
      providers: [
        { provide: UpdaterService, useValue: updater },
        { provide: AppInfoService, useValue: new FakeAppInfo() },
        { provide: ChangelogService, useValue: new FakeChangelog() },
        provideTranslocoTesting(),
      ],
    });
    store = TestBed.inject(UpdateStore);
    fixture = TestBed.createComponent(AboutMenuComponent);
    fixture.autoDetectChanges();
  });

  describe('the update dot', () => {
    const dot = (): HTMLElement | null => fixture.nativeElement.querySelector('[data-testid="update-dot"]');

    it('is absent while there is nothing waiting', () => {
      expect(dot()).toBeNull();
    });

    /** `dismiss()` moves the status to `idle`: a dot reading it would vanish with the prompt. */
    it('stays after the prompt has been dismissed', async () => {
      updater.available = { version: '0.2.0', currentVersion: '0.1.0' };
      await store.check();
      await fixture.whenStable();
      expect(dot()).not.toBeNull();

      await store.dismiss(true);
      await fixture.whenStable();

      expect(dot()).not.toBeNull();
    });

    it('marks the entry the dot is about, once the menu is open', async () => {
      updater.available = { version: '0.2.0', currentVersion: '0.1.0' };
      await store.check();
      await store.dismiss(true);
      await openMenu();

      const marked = option('mises à jour');

      expect(marked.querySelector('[data-testid="update-dot-option"]')).not.toBeNull();
    });

    it('carries a text twin, since it says something no word does', async () => {
      updater.available = { version: '0.2.0', currentVersion: '0.1.0' };
      await store.check();
      await fixture.whenStable();

      expect(trigger().querySelector('.visually-hidden')?.textContent?.trim()).not.toBe('');
    });
  });

  it('keeps the menu closed until asked', () => {
    expect(fixture.debugElement.query(By.css('.about-dropdown'))).toBeNull();
    expect(trigger().getAttribute('aria-expanded')).toBe('false');
  });

  it('opens as a menu and moves focus to its first entry', async () => {
    await openMenu();

    expect(trigger().getAttribute('aria-expanded')).toBe('true');
    expect(fixture.nativeElement.querySelector('.about-dropdown').getAttribute('role')).toBe('menu');
    expect(options()).toHaveLength(5);
    expect(document.activeElement).toBe(options()[0]);
  });

  it('cycles focus through the entries with the arrow keys', async () => {
    await openMenu();
    const panel = fixture.debugElement.query(By.css('.about-dropdown'));

    panel.triggerEventHandler('keydown', new KeyboardEvent('keydown', { key: 'ArrowDown' }));
    expect(document.activeElement).toBe(options()[1]);

    panel.triggerEventHandler('keydown', new KeyboardEvent('keydown', { key: 'ArrowUp' }));
    panel.triggerEventHandler('keydown', new KeyboardEvent('keydown', { key: 'ArrowUp' }));
    expect(document.activeElement).toBe(options().at(-1));
  });

  it('steps the arrow keys over the separators', async () => {
    await openMenu();

    expect(fixture.nativeElement.querySelectorAll('[role="separator"]')).toHaveLength(2);
    expect(fixture.nativeElement.querySelectorAll('[appMenuItem]')).toHaveLength(5);
  });

  it('closes on Escape and hands focus back to the trigger', async () => {
    await openMenu();

    options()[0].dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
    await fixture.whenStable();

    expect(fixture.debugElement.query(By.css('.about-dropdown'))).toBeNull();
    expect(document.activeElement).toBe(trigger());
  });

  it('closes on a click outside itself', async () => {
    await openMenu();

    document.body.click();
    await fixture.whenStable();

    expect(fixture.debugElement.query(By.css('.about-dropdown'))).toBeNull();
  });

  it('reports that the app is up to date, which the startup check never does', async () => {
    await openMenu();

    option('mises à jour').click();
    await fixture.whenStable();

    expect(updater.checkCalls).toBe(1);
    expect(store.checkState()).toBe('upToDate');
    expect(fixture.nativeElement.querySelector('.about-option-status').textContent).toContain('à jour');
  });

  it('announces the status without stealing focus', async () => {
    await openMenu();

    expect(fixture.nativeElement.querySelector('.about-option-status').getAttribute('aria-live')).toBe(
      'polite',
    );
  });

  it('reports a failed check in place', async () => {
    const warn = vi.spyOn(console, 'warn').mockImplementation(() => undefined);
    updater.checkError = new Error('network unreachable');
    await openMenu();

    option('mises à jour').click();
    await fixture.whenStable();

    expect(fixture.nativeElement.querySelector('.about-option-status').textContent).toContain('impossible');
    warn.mockRestore();
  });

  it('ignores a second click while a check is running', async () => {
    updater.deferCheck = true;
    await openMenu();

    option('mises à jour').click();
    await fixture.whenStable();
    option('mises à jour').click();
    await fixture.whenStable();

    expect(option('mises à jour').getAttribute('aria-disabled')).toBe('true');
    expect(updater.checkCalls).toBe(1);

    updater.finishCheck();
    await fixture.whenStable();
  });

  it('opens the about card and closes the menu', async () => {
    await openPanel('À propos de DevNotes');

    expect(fixture.debugElement.query(By.directive(AboutDialogComponent))).not.toBeNull();
    expect(fixture.debugElement.query(By.css('.about-dropdown'))).toBeNull();
  });

  it('opens each help panel from its own entry', async () => {
    await openPanel('Nouveautés');
    expect(fixture.debugElement.query(By.directive(WhatsNewDialogComponent))).not.toBeNull();
    fixture.debugElement.query(By.css('.news-close')).triggerEventHandler('click');
    await fixture.whenStable();

    await openPanel('Prise en main');
    expect(fixture.debugElement.query(By.directive(GettingStartedDialogComponent))).not.toBeNull();
    fixture.debugElement.query(By.css('.guide-close')).triggerEventHandler('click');
    await fixture.whenStable();

    await openPanel('Raccourcis clavier');
    expect(fixture.debugElement.query(By.directive(ShortcutsDialogComponent))).not.toBeNull();
  });

  it('never stacks two panels', async () => {
    await openPanel('Raccourcis clavier');

    expect(fixture.debugElement.query(By.directive(ShortcutsDialogComponent))).not.toBeNull();
    expect(fixture.debugElement.query(By.directive(AboutDialogComponent))).toBeNull();
    expect(fixture.debugElement.query(By.directive(WhatsNewDialogComponent))).toBeNull();
  });

  it('returns focus to the trigger when a panel closes', async () => {
    await openPanel('À propos de DevNotes');

    fixture.debugElement.query(By.css('.about-close')).triggerEventHandler('click');
    await fixture.whenStable();

    expect(fixture.debugElement.query(By.directive(AboutDialogComponent))).toBeNull();
    expect(document.activeElement).toBe(trigger());
  });
});
