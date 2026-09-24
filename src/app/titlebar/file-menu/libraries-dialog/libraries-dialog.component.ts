import { ChangeDetectionStrategy, Component, computed, inject, output, signal } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { LibraryEntry } from '@core/data/libraries.repository';
import { LibrariesStore } from '@core/state/libraries.store';
import { DialogComponent } from '@shared/layout/dialog/dialog.component';

/**
 * The libraries, and the four things one can do with them.
 *
 * Opening one is a full teardown: the connection closes, every command answers
 * `Locked`, and the shell goes back to the gate. So the dialog closes itself on a switch
 * — a panel left standing over the unlock screen would be asking about a library nobody
 * has opened yet.
 */
@Component({
  selector: 'app-libraries-dialog',
  imports: [DialogComponent, TranslocoPipe],
  templateUrl: './libraries-dialog.component.html',
  styleUrl: './libraries-dialog.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class LibrariesDialogComponent {
  readonly closed = output<void>();

  protected readonly store = inject(LibrariesStore);

  protected readonly newName = signal('');
  protected readonly renaming = signal<string | null>(null);
  protected readonly renamedTo = signal('');

  protected readonly rows = computed(() =>
    this.store.libraries().map((entry) => ({
      entry,
      isOpen: entry.id === this.store.open()?.id,
      // The last one cannot go: the gate would have nothing to offer, and the next
      // read of the registry would adopt an empty profile as a library nobody asked for.
      isOnlyOne: !this.store.hasSeveral(),
    })),
  );

  protected name(entry: LibraryEntry): string {
    return entry.name.trim();
  }

  protected async create(): Promise<void> {
    const name = this.newName().trim();
    if (name === '') return;

    this.newName.set('');
    await this.store.create(name);
    // Creating opens it, and opening sends the shell to the gate: a panel left standing
    // over the unlock screen would be asking about a library nobody has opened yet.
    this.closed.emit();
  }

  protected async openLibrary(entry: LibraryEntry): Promise<void> {
    if (entry.id === this.store.open()?.id) return;

    await this.store.openLibrary(entry.id);
    this.closed.emit();
  }

  protected startRenaming(entry: LibraryEntry): void {
    this.renaming.set(entry.id);
    this.renamedTo.set(this.name(entry));
  }

  protected async commitRename(): Promise<void> {
    const id = this.renaming();
    const name = this.renamedTo().trim();
    this.renaming.set(null);
    if (id === null || name === '') return;

    await this.store.rename(id, name);
  }

  protected cancelRename(): void {
    this.renaming.set(null);
  }
}
