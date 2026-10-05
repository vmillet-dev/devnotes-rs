import {
  ChangeDetectionStrategy,
  Component,
  ComponentRef,
  ViewContainerRef,
  computed,
  effect,
  inject,
  input,
  resource,
  signal,
  untracked,
  viewChild,
} from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { ErrorNotifier } from '@core/services/errors/error-notifier.service';
import { StatusNotifier } from '@core/services/notifications/status.service';
import { Tool, ToolDefinition, ToolResult } from '@core/services/tools/tool.model';
import { ToolsStore } from '@core/services/tools/tools.store';
import { SaveAsNoteDialogComponent } from './save-as-note/save-as-note-dialog.component';

/**
 * What every tool is drawn in: the breadcrumb, the tool's own actions, Vider and "Enregistrer
 * comme note", which keeps the tool's result — never what was typed into it. The tool is
 * created here by hand rather than through `NgComponentOutlet`, so what it exposes is a signal
 * the header can read the moment it exists.
 */
@Component({
  selector: 'app-tool-frame',
  imports: [SaveAsNoteDialogComponent, TranslocoPipe],
  templateUrl: './tool-frame.component.html',
  styleUrl: './tool-frame.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ToolFrameComponent {
  protected readonly store = inject(ToolsStore);
  private readonly notifier = inject(ErrorNotifier);
  private readonly status = inject(StatusNotifier);

  readonly tool = input.required<ToolDefinition>();

  private readonly host = viewChild.required('host', { read: ViewContainerRef });

  private readonly component = resource({
    params: () => this.tool(),
    loader: ({ params }) => params.load(),
  });

  private readonly created = signal<ComponentRef<Tool> | null>(null);

  protected readonly actions = computed(() => this.created()?.instance.actions?.() ?? []);
  protected readonly result = computed(() => this.created()?.instance.result() ?? null);
  protected readonly hasSample = computed(() => typeof this.created()?.instance.sample === 'function');

  /** Taken when the dialog opens: typing on underneath does not change what is being saved. */
  protected readonly saving = signal<ToolResult | null>(null);

  constructor() {
    effect((onCleanup) => {
      const type = this.component.hasValue() ? this.component.value() : undefined;
      const host = this.host();
      if (type === undefined) return;

      untracked(() => {
        const ref = host.createComponent(type);
        this.created.set(ref);
        onCleanup(() => {
          this.created.set(null);
          ref.destroy();
        });
      });
    });

    effect(() => {
      if (this.component.status() === 'error') {
        this.notifier.reportFailure('errors.toolFailed', this.component.error());
      }
    });
  }

  /** `aria-disabled` rather than `disabled`: a disabled button shows no tooltip, and this one says why. */
  protected saveAsNote(): void {
    const result = this.result();
    if (result !== null) {
      this.saving.set(result);
    }
  }

  protected sample(): void {
    this.created()?.instance.sample?.();
    this.status.notify({ key: 'tools.sampleLoaded' });
  }

  protected clear(): void {
    this.created()?.instance.clear();
  }
}
