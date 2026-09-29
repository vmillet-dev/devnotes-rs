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
import { Tool, ToolDefinition } from '@core/services/tools/tool.model';
import { ToolsStore } from '@core/services/tools/tools.store';

/**
 * What every tool is drawn in: the breadcrumb, the tool's own actions, Vider. The tool is
 * created here by hand rather than through `NgComponentOutlet`, so what it exposes is a signal
 * the header can read the moment it exists.
 */
@Component({
  selector: 'app-tool-frame',
  imports: [TranslocoPipe],
  templateUrl: './tool-frame.component.html',
  styleUrl: './tool-frame.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ToolFrameComponent {
  protected readonly store = inject(ToolsStore);
  private readonly notifier = inject(ErrorNotifier);

  readonly tool = input.required<ToolDefinition>();

  private readonly host = viewChild.required('host', { read: ViewContainerRef });

  private readonly component = resource({
    params: () => this.tool(),
    loader: ({ params }) => params.load(),
  });

  private readonly created = signal<ComponentRef<Tool> | null>(null);

  protected readonly actions = computed(() => this.created()?.instance.actions?.() ?? []);

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

  protected clear(): void {
    this.created()?.instance.clear();
  }
}
