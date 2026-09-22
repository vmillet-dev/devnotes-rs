import { Injectable, computed, inject, signal } from '@angular/core';
import { ErrorNotifier } from '@core/services/errors/error-notifier.service';
import { NotesRepository } from '../data/notes.repository';
import { Variable, duplicateNames, toVariableRecord } from '../model/variable.model';

/**
 * ⚠️ The local state is a list, not the map the back end returns: a row just added has
 * neither name nor value yet, and a map would lose it on the next keystroke.
 */
@Injectable({ providedIn: 'root' })
export class VariablesStore {
  private readonly repository = inject(NotesRepository);
  private readonly notifier = inject(ErrorNotifier);

  private readonly _variables = signal<readonly Variable[]>([]);
  private readonly _isLoading = signal(false);
  private readonly _isDirty = signal(false);

  readonly variables = this._variables.asReadonly();
  readonly isLoading = this._isLoading.asReadonly();

  /**
   * ⚠️ These rows are edited in the preferences panel, which commits on a button — so
   * nothing here reaches the corpus until the panel says so, and this is what tells it
   * there is something waiting.
   */
  readonly isDirty = this._isDirty.asReadonly();
  readonly isEmpty = computed(() => !this._isLoading() && this._variables().length === 0);

  readonly duplicates = computed(() => duplicateNames(this._variables()));

  /**
   * ⚠️ Refuses to overwrite edits in hand. The page is recreated every time the rail
   * changes section, so without this, leaving the page and coming back would silently
   * drop what was typed and not yet applied.
   */
  async load(): Promise<void> {
    if (this._isDirty()) return;

    this._isLoading.set(true);
    try {
      const stored = await this.repository.loadVariables();
      this._variables.set(Object.entries(stored).map(([name, value]) => ({ name, value })));
      this._isDirty.set(false);
    } catch (error) {
      this.notifier.reportFailure('errors.variablesLoadFailed', error);
    } finally {
      this._isLoading.set(false);
    }
  }

  add(): void {
    this._variables.update((variables) => [...variables, { name: '', value: '' }]);
    this._isDirty.set(true);
  }

  rename(index: number, name: string): void {
    this.replace(index, (variable) => ({ ...variable, name: name.trim() }));
  }

  setValue(index: number, value: string): void {
    this.replace(index, (variable) => ({ ...variable, value }));
  }

  remove(index: number): void {
    this._variables.update((variables) => variables.filter((_, position) => position !== index));
    this._isDirty.set(true);
  }

  /** Drops what was typed and reads the corpus again, which is what Annuler means here. */
  async discard(): Promise<void> {
    this._isDirty.set(false);
    await this.load();
  }

  /**
   * Sends the whole set. What the back end keeps is not adopted back: it drops
   * half-filled rows, which have to stay on screen long enough to be finished.
   */
  async commit(): Promise<void> {
    this._isDirty.set(false);
    await this.notifier.attempt('errors.variablesSaveFailed', () =>
      this.repository.saveVariables(toVariableRecord(this._variables())),
    );
  }

  private replace(index: number, change: (variable: Variable) => Variable): void {
    this._isDirty.set(true);
    this._variables.update((variables) =>
      variables.map((variable, position) => (position === index ? change(variable) : variable)),
    );
  }
}
