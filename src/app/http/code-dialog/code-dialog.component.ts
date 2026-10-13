import {
  ChangeDetectionStrategy,
  Component,
  computed,
  inject,
  input,
  output,
  resource,
  signal,
} from '@angular/core';
import { TranslocoPipe, TranslocoService } from '@jsverse/transloco';
import { HttpRepository } from '@core/data/http.repository';
import {
  CODE_GROUPS,
  CODE_TARGETS,
  CodeGroup,
  CodeOptions,
  CodeTarget,
  CodeTargetEntry,
  GeneratedCode,
  matchesFilter,
} from '@core/model/http-code.model';
import { requestBadge } from '@core/model/http.model';
import { ClipboardService } from '@core/services/clipboard/clipboard.service';
import { HttpEnvironmentsStore } from '@core/services/http/http-environments.store';
import { RequestTab } from '@core/services/http/http-tabs.store';
import { StatusNotifier } from '@core/services/notifications/status.service';
import { ToolResult } from '@core/services/tools/tool.model';
import { SaveAsNoteDialogComponent } from '@app/save-as-note/save-as-note-dialog.component';
import { CodeViewerComponent } from '@shared/code-viewer/code-viewer.component';
import { DialogComponent } from '@shared/layout/dialog/dialog.component';
import { IconComponent } from '@shared/icon/icon.component';

type Switch = 'resolveVariables' | 'multiline' | 'followRedirects';

/** The target last chosen, for the session: the dialog opens again on it. */
let lastTarget: CodeTarget = 'curl';

/**
 * « Générer le code »: the request written for another tool, from what a send would write.
 * Rust writes it; the dialog chooses the target and the switches, and says what was left as a
 * variable. « Inclure les secrets » asks every time it is switched on.
 */
@Component({
  selector: 'app-code-dialog',
  imports: [CodeViewerComponent, DialogComponent, IconComponent, SaveAsNoteDialogComponent, TranslocoPipe],
  templateUrl: './code-dialog.component.html',
  styleUrl: './code-dialog.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class CodeDialogComponent {
  readonly tab = input.required<RequestTab>();
  readonly closed = output<void>();

  private readonly repository = inject(HttpRepository);
  private readonly clipboard = inject(ClipboardService);
  private readonly status = inject(StatusNotifier);
  private readonly transloco = inject(TranslocoService);
  protected readonly environments = inject(HttpEnvironmentsStore);

  protected readonly target = signal<CodeTarget>(lastTarget);
  protected readonly filter = signal('');
  protected readonly options = signal<CodeOptions>({
    resolveVariables: true,
    includeSecrets: false,
    multiline: true,
    followRedirects: false,
  });
  /** « Inclure les secrets » switched on, waiting for its yes. */
  protected readonly askingSecrets = signal(false);
  protected readonly saving = signal<ToolResult | null>(null);

  protected readonly badge = computed(() => requestBadge(this.tab().draft));
  protected readonly groups = computed(() =>
    CODE_GROUPS.map((group) => ({
      group,
      targets: CODE_TARGETS.filter((entry) => entry.group === group && matchesFilter(entry, this.filter())),
    })).filter(({ targets }) => targets.length > 0),
  );
  protected readonly entry = computed<CodeTargetEntry>(
    () => CODE_TARGETS.find((entry) => entry.id === this.target()) ?? CODE_TARGETS[0]!,
  );

  private readonly codeResource = resource({
    params: () => ({
      target: this.target(),
      options: this.options(),
      draft: this.tab().draft,
      place: this.tab().place,
      environmentId: this.environments.active()?.id ?? null,
    }),
    loader: ({ params }) =>
      this.repository.generateCode(
        params.target,
        params.options,
        params.draft,
        params.place,
        params.environmentId,
      ),
  });
  protected readonly code = computed<GeneratedCode | null>(() =>
    this.codeResource.hasValue() ? (this.codeResource.value() ?? null) : null,
  );

  protected groupKey(group: CodeGroup): string {
    return `http.code.groups.${group}`;
  }

  protected choose(target: CodeTarget): void {
    lastTarget = target;
    this.target.set(target);
  }

  protected onFilter(event: Event): void {
    this.filter.set((event.target as HTMLInputElement).value);
  }

  protected toggle(name: Switch): void {
    this.options.update((options) => ({ ...options, [name]: !options[name] }));
  }

  /**
   * On, it asks first — the code then holds what the library seals —; off, it never asks. The
   * box shows what is in force, not the click, until the yes.
   */
  protected onSecrets(event: Event): void {
    if (this.options().includeSecrets) {
      this.options.update((options) => ({ ...options, includeSecrets: false }));
    } else {
      this.askingSecrets.set(true);
    }
    (event.target as HTMLInputElement).checked = this.options().includeSecrets;
  }

  /** A name as the request writes it, braces and all. */
  protected written(name: string): string {
    return `{{${name}}}`;
  }

  protected includeSecrets(): void {
    this.askingSecrets.set(false);
    this.options.update((options) => ({ ...options, includeSecrets: true }));
  }

  protected async copy(): Promise<void> {
    const code = this.code();
    if (code === null) return;
    if (await this.clipboard.copy(code.code)) this.status.notify({ key: 'http.code.copied' });
  }

  protected saveAsSnippet(): void {
    const code = this.code();
    if (code === null) return;
    this.saving.set({
      title: {
        key: 'http.code.noteTitle',
        params: { name: this.tab().draft.name, target: this.entry().name },
      },
      kind: 'snippet',
      language: code.language,
      content: code.code,
    });
  }

  protected noteSource(): string {
    return `${this.transloco.translate('areas.http')} / ${this.tab().draft.name}`;
  }
}
