import {
  ChangeDetectionStrategy,
  Component,
  ElementRef,
  Injector,
  afterNextRender,
  computed,
  effect,
  inject,
  untracked,
  viewChild,
} from '@angular/core';
import { toSignal } from '@angular/core/rxjs-interop';
import { TranslocoPipe, TranslocoService } from '@jsverse/transloco';
import { ClockService } from '@core/services/time/clock.service';
import {
  TOOL_CATALOGUE,
  TOOL_CATEGORIES,
  ToolCategory,
  ToolDefinition,
  toolMatches,
} from '@core/services/tools/tool.model';
import { ToolsStore } from '@core/services/tools/tools.store';
import { relativeTimeRef } from '@core/utils/relative-time.util';
import { IconComponent, IconName } from '@shared/icon/icon.component';

const CATEGORY_ICONS: Record<ToolCategory, IconName> = {
  text: 'type',
  crypto: 'lock',
  encode: 'code',
  generate: 'sparkle',
  time: 'clock',
  calc: 'calculator',
};

/** The catalogue in its panels, the recent tools above it, and a search over both names and lines. */
@Component({
  selector: 'app-tools-home',
  imports: [IconComponent, TranslocoPipe],
  templateUrl: './tools-home.component.html',
  styleUrl: './tools-home.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ToolsHomeComponent {
  protected readonly store = inject(ToolsStore);
  protected readonly catalogue = inject(TOOL_CATALOGUE);
  private readonly transloco = inject(TranslocoService);
  private readonly clock = inject(ClockService);
  private readonly injector = inject(Injector);

  private readonly searchField = viewChild.required<ElementRef<HTMLInputElement>>('searchField');

  /** The names are searched in the language on screen: its file loaded, the search runs again. */
  private readonly translation = toSignal(this.transloco.selectTranslation());

  protected readonly searching = computed(() => this.store.search().trim() !== '');

  protected readonly panels = computed(() => {
    this.translation();
    const query = this.store.search();
    const only = this.searching() ? null : this.store.category();

    return TOOL_CATEGORIES.filter((category) => only === null || category === only)
      .map((category) => ({
        category,
        icon: CATEGORY_ICONS[category],
        tools: this.catalogue.filter((tool) => tool.category === category && this.matches(tool, query)),
      }))
      .filter((panel) => panel.tools.length > 0);
  });

  protected readonly recents = computed(() =>
    this.store.recents().flatMap((recent) => {
      const tool = this.catalogue.find((candidate) => candidate.id === recent.id);
      return tool ? [{ tool, when: relativeTimeRef(new Date(recent.at), this.clock.now()) }] : [];
    }),
  );

  constructor() {
    // Ctrl+Shift+T may arrive before this home exists — from the notes — or while it shows.
    effect(() => {
      if (!this.store.searchWanted()) return;

      untracked(() => {
        this.store.searchTaken();
        afterNextRender(() => this.searchField().nativeElement.focus(), { injector: this.injector });
      });
    });
  }

  protected onSearch(event: Event): void {
    this.store.setSearch((event.target as HTMLInputElement).value);
  }

  private matches(tool: ToolDefinition, query: string): boolean {
    return toolMatches(query, [
      this.transloco.translate(`tools.${tool.id}.name`),
      this.transloco.translate(`tools.${tool.id}.description`),
      ...tool.keywords,
    ]);
  }
}
