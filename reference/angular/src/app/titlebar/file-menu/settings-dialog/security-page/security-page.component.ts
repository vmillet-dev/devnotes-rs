import {
  ChangeDetectionStrategy,
  Component,
  ElementRef,
  PendingTasks,
  afterRenderEffect,
  computed,
  inject,
  signal,
  viewChild,
} from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { Backup } from '@core/model/backup.model';
import { SettingsDraftStore } from '@core/services/settings/settings-draft.store';
import { ClockService } from '@core/services/time/clock.service';
import { BackupsStore } from '@core/state/backups.store';
import { relativeTimeRef } from '@core/utils/relative-time.util';
import { ChangePassphraseDialogComponent } from '../change-passphrase-dialog/change-passphrase-dialog.component';

/** Rounded to the unit that reads: a copy is megabytes, not 4 233 981 bytes. */
function humanSize(bytes: number): string {
  const mega = bytes / (1024 * 1024);

  return mega >= 1 ? `${mega.toFixed(1)} Mo` : `${Math.max(1, Math.round(bytes / 1024))} Ko`;
}

/**
 * The two things that decide who can read the library — the phrase that unwraps its key,
 * and whether a copy is taken at every launch — and the copies themselves.
 *
 * They were invisible: taken at unlock, kept beside the library, pruned to three, and
 * none of it said anywhere the application could be read from. A safety net nobody can
 * see is one nobody trusts, and one nobody can use.
 */
@Component({
  selector: 'app-security-page',
  imports: [TranslocoPipe, ChangePassphraseDialogComponent],
  templateUrl: './security-page.component.html',
  styleUrl: './security-page.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class SecurityPageComponent {
  protected readonly draft = inject(SettingsDraftStore);
  protected readonly backups = inject(BackupsStore);

  private readonly clock = inject(ClockService);

  protected readonly isChangingPassphrase = signal(false);

  /** Formatted here rather than in Rust: a label has to age without a round trip. */
  protected readonly copies = computed(() =>
    this.backups.backups().map((backup) => ({
      backup,
      size: humanSize(backup.bytes),
      when: relativeTimeRef(backup.takenAt, this.clock.now()),
    })),
  );

  private readonly confirmStrip = viewChild<ElementRef<HTMLElement>>('confirmStrip');

  constructor() {
    // A pending task, so `whenStable` waits for the read rather than for a guess at it.
    void inject(PendingTasks).run(() => this.backups.load());

    // The strip replaces the trigger further down a panel that scrolls, so without
    // this the click reads as having done nothing at all. `?.` on the call because jsdom
    // has no `scrollIntoView`.
    afterRenderEffect(() => {
      this.confirmStrip()?.nativeElement.scrollIntoView?.({ block: 'nearest' });
    });
  }

  protected openPassphraseChange(): void {
    this.isChangingPassphrase.set(true);
  }

  protected closePassphraseChange(): void {
    this.isChangingPassphrase.set(false);
  }

  protected onAutomaticBackups(event: Event): void {
    this.draft.set('automaticBackups', (event.target as HTMLInputElement).checked);
  }

  protected ask(backup: Backup): void {
    this.backups.ask(backup);
  }
}
