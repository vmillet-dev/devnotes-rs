import {
  ChangeDetectionStrategy,
  Component,
  ElementRef,
  OnInit,
  computed,
  inject,
  signal,
  viewChild,
} from '@angular/core';
import { TranslocoPipe, TranslocoService } from '@jsverse/transloco';
import type { FolderColour } from '@core/ipc/bindings';
import {
  ENVIRONMENT_COLOURS,
  FilledBy,
  HttpEnvironment,
  VariableRow,
  variableCounts,
} from '@core/model/http-environment.model';
import { HttpEnvironmentsStore } from '@core/services/http/http-environments.store';
import { HttpScriptFocusStore } from '@core/services/http/http-script-focus.store';
import { HttpTabsStore } from '@core/services/http/http-tabs.store';
import { HttpWorkspaceStore } from '@core/services/http/http-workspace.store';
import { SettingsStore } from '@core/services/settings/settings.store';
import { RailToggleComponent } from '@shared/controls/rail-toggle/rail-toggle.component';
import { IconComponent } from '@shared/icon/icon.component';
import { VariablesTableComponent } from '../ui/variables-table/variables-table.component';

/**
 * The globals and the environments, in place of the requests while it is open: the list on the
 * left, the set chosen on the right — its name, colour and protection, and its variables, written
 * as they are typed.
 */
@Component({
  selector: 'app-environments-page',
  imports: [IconComponent, RailToggleComponent, TranslocoPipe, VariablesTableComponent],
  templateUrl: './environments-page.component.html',
  styleUrl: './environments-page.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class EnvironmentsPageComponent implements OnInit {
  protected readonly store = inject(HttpEnvironmentsStore);
  protected readonly settings = inject(SettingsStore);
  private readonly transloco = inject(TranslocoService);
  private readonly tabs = inject(HttpTabsStore);
  private readonly workspace = inject(HttpWorkspaceStore);
  private readonly scriptFocus = inject(HttpScriptFocusStore);

  protected readonly colours = ENVIRONMENT_COLOURS;
  protected readonly creating = signal(false);
  protected readonly confirmingDelete = signal(false);
  private readonly createField = viewChild<ElementRef<HTMLInputElement>>('createField');

  /** Keys the table by the set it shows: a secret shown in one is not shown in the next. */
  protected readonly tableKey = computed(() => {
    const target = this.store.target();
    return target.kind === 'globals' ? 'globals' : target.id;
  });

  ngOnInit(): void {
    void this.store.load();
  }

  /** The request that fills a variable, open on its scripts after the answer. */
  protected async openFiller(by: FilledBy): Promise<void> {
    if (!by.requestId) return;
    await this.store.flush();
    await this.tabs.open(by.requestId);
    this.workspace.show('requests');
    this.scriptFocus.show(by.requestId, { phase: 'after', line: null });
  }

  protected counts(variables: readonly VariableRow[]): { readonly count: number; readonly secrets: number } {
    return variableCounts(variables);
  }

  protected isShown(environment: HttpEnvironment): boolean {
    const target = this.store.target();
    return target.kind === 'environment' && target.id === environment.id;
  }

  protected select(environment: HttpEnvironment | null): void {
    this.confirmingDelete.set(false);
    this.store.select(
      environment === null ? { kind: 'globals' } : { kind: 'environment', id: environment.id },
    );
  }

  protected startCreate(): void {
    this.creating.set(true);
    queueMicrotask(() => this.createField()?.nativeElement.focus());
  }

  protected async submitCreate(event: Event, name: string): Promise<void> {
    event.preventDefault();
    const trimmed = name.trim();
    if (trimmed === '') {
      this.creating.set(false);
      return;
    }
    if (await this.store.create(trimmed)) this.creating.set(false);
  }

  /** On leaving the field or Enter: a blank name or the same one writes nothing. */
  protected rename(environment: HttpEnvironment, event: Event): void {
    const field = event.target as HTMLInputElement;
    const name = field.value.trim();
    if (name === '' || name === environment.name) {
      field.value = environment.name;
      return;
    }
    void this.store.rename(environment.id, name).then((renamed) => {
      if (!renamed) field.value = environment.name;
    });
  }

  protected blur(event: Event): void {
    (event.target as HTMLElement).blur();
  }

  protected recolour(environment: HttpEnvironment, colour: FolderColour): void {
    if (environment.colour !== colour) void this.store.recolour(environment.id, colour);
  }

  protected protect(environment: HttpEnvironment, event: Event): void {
    void this.store.protect(environment.id, (event.target as HTMLInputElement).checked);
  }

  protected duplicate(environment: HttpEnvironment): void {
    const name = this.transloco.translate('http.environments.copyName', { name: environment.name });
    void this.store.duplicate(environment.id, name);
  }

  protected async delete(environment: HttpEnvironment): Promise<void> {
    if (await this.store.delete(environment.id)) this.confirmingDelete.set(false);
  }
}
