import { ComponentFixture, TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';
import { SettingsDraftStore } from '@core/services/settings/settings-draft.store';
import { SettingsStore } from '@core/services/settings/settings.store';
import { Backup } from '@core/model/backup.model';
import { FakeBackupsRepository } from '@testing/fake-backups-repository';
import { provideAppTesting } from '@testing/testing.providers';
import { SecurityPageComponent } from './security-page.component';

/** Two copies, as a profile really holds them: one taken today, one the day before. */
const TAKEN: readonly Backup[] = [
  {
    id: '2026-07-25_09-00-00',
    takenAt: new Date('2026-07-25T09:00:00.000Z'),
    bytes: 2_500_000,
    openable: true,
    attachments: true,
  },
  {
    id: '2026-07-24_09-00-00',
    takenAt: new Date('2026-07-24T09:00:00.000Z'),
    bytes: 2_400_000,
    openable: false,
    attachments: false,
  },
];

describe('SecurityPageComponent', () => {
  let fixture: ComponentFixture<SecurityPageComponent>;
  let settings: SettingsStore;
  let draft: SettingsDraftStore;
  let copies: FakeBackupsRepository;

  const backups = (): HTMLInputElement =>
    fixture.nativeElement.querySelector('[data-testid="setting-automatic-backups"]');

  const attachments = (): HTMLInputElement =>
    fixture.nativeElement.querySelector('[data-testid="setting-backup-attachments"]');

  beforeEach(async () => {
    TestBed.resetTestingModule();
    copies = new FakeBackupsRepository(TAKEN);
    TestBed.configureTestingModule({
      imports: [SecurityPageComponent],
      providers: [provideAppTesting({ backupsRepository: copies })],
    });
    settings = TestBed.inject(SettingsStore);
    draft = TestBed.inject(SettingsDraftStore);
    draft.cancel();
    fixture = TestBed.createComponent(SecurityPageComponent);
    fixture.autoDetectChanges();
    await fixture.whenStable();
  });

  /** The dialog is opened from here and nowhere else; what it does is its own spec. */
  it('opens the passphrase dialog, and only on demand', async () => {
    const opener = (): HTMLButtonElement =>
      fixture.nativeElement.querySelector('[data-testid="setting-change-passphrase"]');
    const dialog = (): HTMLElement | null =>
      fixture.nativeElement.querySelector('[data-testid="change-passphrase"]');

    expect(dialog()).toBeNull();

    opener().click();
    await fixture.whenStable();

    expect(dialog()).not.toBeNull();
  });

  /**
   * Rust reads this at launch, before the front end exists — turning it off has to
   * reach the preferences file, which is the only thing the back end sees. Which is
   * exactly why it waits for Appliquer: until then the switch is a draft.
   */
  it('shows the copies as on, and stages the choice until it is applied', async () => {
    expect(backups().checked).toBe(true);

    backups().click();
    await fixture.whenStable();

    expect(backups().checked).toBe(false);
    expect(draft.value('automaticBackups')).toBe(false);
    expect(settings.automaticBackups()).toBe(true);

    draft.apply();

    expect(settings.automaticBackups()).toBe(false);
  });

  /** Read by Rust at launch like the switch above it, so it waits for Appliquer too. */
  it('stages whether a copy carries the attachments until it is applied', async () => {
    expect(attachments().checked).toBe(true);

    attachments().click();
    await fixture.whenStable();

    expect(draft.value('backupAttachments')).toBe(false);
    expect(settings.backupAttachments()).toBe(true);

    draft.apply();

    expect(settings.backupAttachments()).toBe(false);
  });

  /** It decides what a copy holds, and with no copy taken there is nothing to decide. */
  it('greys the attachments switch while no copy is taken', async () => {
    expect(attachments().disabled).toBe(false);

    backups().click();
    await fixture.whenStable();

    expect(attachments().disabled).toBe(true);
  });

  /**
   * The two together, which is the whole of this page: the phrase unwraps the key and
   * the copies are wrapped under it too, so a page carrying only one of them would let
   * someone change the phrase without meeting what the change reaches.
   */
  it('names the copies beside the phrase, not somewhere else', () => {
    const titles = [...fixture.nativeElement.querySelectorAll('.setting-group-title')].map(
      (title: HTMLElement) => title.textContent?.trim(),
    );

    expect(titles).toEqual(['Accès', 'Copies de sauvegarde']);
  });
  const rows = (): HTMLElement[] => [...fixture.nativeElement.querySelectorAll('[data-testid="backup-row"]')];

  const restoreButton = (id: string): HTMLButtonElement | null =>
    fixture.nativeElement.querySelector(`[data-testid="backup-restore"][data-backup="${id}"]`);

  const confirmStrip = (): HTMLElement | null =>
    fixture.nativeElement.querySelector('[data-testid="backup-confirm"]');

  /** Taken at unlock, kept beside the library, pruned to three: said where it can be read. */
  describe('the copies it lists', () => {
    it('shows one row per copy, with when it was taken and how big it is', () => {
      expect(rows()).toHaveLength(2);
      expect(rows()[0].textContent).toContain('2026-07-25_09-00-00');
      expect(rows()[0].textContent).toContain('2.4 Mo');
    });

    /** Restoring one of those leaves the live attachments in place, which is worth knowing first. */
    it('says which copies were taken without the attachments', () => {
      expect(rows()[0].textContent).not.toContain('sans pièces jointes');
      expect(rows()[1].textContent).toContain('sans pièces jointes');
    });

    /** Listed, never offered: it opens for nobody. */
    it('offers no restore on a copy whose key file did not travel with it', () => {
      expect(restoreButton('2026-07-25_09-00-00')).not.toBeNull();
      expect(restoreButton('2026-07-24_09-00-00')).toBeNull();
      expect(rows()[1].textContent).toContain('illisible');
    });

    /**
     * The trigger is replaced by a sentence naming what will happen, and the confirm
     * sits somewhere else: a second click on the button that fired it is the guard a
     * double click defeats.
     */
    it('names what a restore would do before it runs one', async () => {
      expect(confirmStrip()).toBeNull();

      restoreButton('2026-07-25_09-00-00')!.click();
      await fixture.whenStable();

      expect(confirmStrip()?.textContent).toContain('2026-07-25_09-00-00');
      expect(restoreButton('2026-07-25_09-00-00')).toBeNull();
      expect(copies.restored).toEqual([]);
    });

    it('runs it only from the button that is not the one that was clicked', async () => {
      restoreButton('2026-07-25_09-00-00')!.click();
      await fixture.whenStable();

      (
        fixture.nativeElement.querySelector('[data-testid="backup-confirm-restore"]') as HTMLButtonElement
      ).click();
      await fixture.whenStable();

      expect(copies.restored).toEqual(['2026-07-25_09-00-00']);
    });

    it('gives up on it without running anything', async () => {
      restoreButton('2026-07-25_09-00-00')!.click();
      await fixture.whenStable();

      (fixture.nativeElement.querySelector('[data-testid="backup-cancel"]') as HTMLButtonElement).click();
      await fixture.whenStable();

      expect(confirmStrip()).toBeNull();
      expect(copies.restored).toEqual([]);
    });
  });
});
