import {
  ChangeDetectionStrategy,
  Component,
  DestroyRef,
  ElementRef,
  Injector,
  afterNextRender,
  computed,
  effect,
  inject,
  signal,
  untracked,
  viewChild,
} from '@angular/core';
import { toSignal } from '@angular/core/rxjs-interop';
import { TranslocoPipe, TranslocoService } from '@jsverse/transloco';
import { SettingsStore } from '@core/services/settings/settings.store';
import { ClockService } from '@core/services/time/clock.service';
import { ReferenceHit, referenceSearch } from '@core/services/tools/reference-search';
import { referenceQueryKey } from '@core/services/tools/reference.model';
import {
  FAVOURITES,
  MAX_RECENT_TOOLS,
  TOOL_CATALOGUE,
  TOOL_CATEGORIES,
  ToolDefinition,
  ToolFilter,
  favouritesAmong,
  toolMatches,
} from '@core/services/tools/tool.model';
import { ToolSessions } from '@core/services/tools/tool-sessions';
import { ToolsStore } from '@core/services/tools/tools.store';
import { columnCount, stackInColumns } from '@core/utils/columns.util';
import { relativeTimeRef } from '@core/utils/relative-time.util';
import { RailToggleComponent } from '@shared/controls/rail-toggle/rail-toggle.component';
import { IconComponent } from '@shared/icon/icon.component';
import { TOOL_CATEGORY_ICONS } from '@tools/catalogue/catalogue';

const PANEL_MIN_WIDTH = 260;
const PANEL_GAP = 14;

/**
 * The catalogue in its panels, the recent tools above it, and a search over the tools — their
 * names, lines, keywords and category — and, under them, inside the references.
 */
@Component({
  selector: 'app-tools-home',
  imports: [IconComponent, RailToggleComponent, TranslocoPipe],
  templateUrl: './tools-home.component.html',
  styleUrl: './tools-home.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
  host: { '(keydown)': 'onKey($event)' },
})
export class ToolsHomeComponent {
  protected readonly store = inject(ToolsStore);
  protected readonly catalogue = inject(TOOL_CATALOGUE);
  protected readonly settings = inject(SettingsStore);
  private readonly transloco = inject(TranslocoService);
  private readonly clock = inject(ClockService);
  private readonly injector = inject(Injector);
  private readonly sessions = inject(ToolSessions);
  private readonly host = inject<ElementRef<HTMLElement>>(ElementRef);
  private readonly destroyRef = inject(DestroyRef);

  private readonly searchField = viewChild.required<ElementRef<HTMLInputElement>>('searchField');
  private readonly panelsBox = viewChild.required<ElementRef<HTMLElement>>('panelsBox');

  protected readonly panelGap = PANEL_GAP;
  private readonly panelsWidth = signal(0);
  protected readonly columnCount = computed(() =>
    columnCount(this.panelsWidth(), PANEL_MIN_WIDTH, PANEL_GAP),
  );

  /** The names are searched in the language on screen: its file loaded, the search runs again. */
  private readonly translation = toSignal(this.transloco.selectTranslation());

  protected readonly searching = computed(() => this.store.search().trim() !== '');

  /** In the order they were starred; an id the catalogue no longer has is dropped. */
  protected readonly favourites = computed(() => favouritesAmong(this.catalogue, this.store.favourites()));
  protected readonly starred = computed(() => new Set(this.favourites().map((tool) => tool.id)));

  /** What the home is narrowed to, a search aside: the favourites only while there is one. */
  private readonly only = computed<ToolFilter | null>(() => {
    const filter = this.store.category();
    return filter === FAVOURITES && this.favourites().length === 0 ? null : filter;
  });

  /** The filter showing while nothing on screen says so: the rail is hidden, and no search overrides it. */
  protected readonly hiddenCategory = computed(() =>
    this.settings.showLibraryRail() || this.searching() ? null : this.only(),
  );

  protected readonly panels = computed(() => {
    this.translation();
    const query = this.store.search();
    const only = this.searching() ? null : this.only();
    const starred = this.starred();

    return TOOL_CATEGORIES.filter((category) => only === null || only === FAVOURITES || category === only)
      .map((category) => ({
        category,
        icon: TOOL_CATEGORY_ICONS[category],
        tools: this.catalogue.filter(
          (tool) =>
            tool.category === category &&
            (only !== FAVOURITES || starred.has(tool.id)) &&
            this.matches(tool, query),
        ),
      }))
      .filter((panel) => panel.tools.length > 0);
  });

  /** A panel's height counted in rows, its head one of them: near enough to share the columns out. */
  protected readonly columns = computed(() =>
    stackInColumns(this.panels(), (panel) => panel.tools.length + 1, this.columnCount()),
  );

  /**
   * More are kept than shown, for the rail: here, the five opened last — a favourite aside,
   * which has its chip above already.
   */
  protected readonly recents = computed(() => {
    const starred = this.starred();
    return this.store
      .recents()
      .flatMap((recent) => {
        const tool = this.catalogue.find((candidate) => candidate.id === recent.id);
        return tool && !starred.has(tool.id)
          ? [{ tool, when: relativeTimeRef(new Date(recent.at), this.clock.now()) }]
          : [];
      })
      .slice(0, MAX_RECENT_TOOLS);
  });

  private readonly inReferences = referenceSearch(this.catalogue, this.store.search);
  protected readonly references = this.inReferences.found;

  /** Nothing matches, and nothing is still on its way that could. */
  protected readonly noMatch = computed(
    () =>
      this.searching() &&
      this.panels().length === 0 &&
      this.references().length === 0 &&
      !this.inReferences.loading(),
  );

  /** What Enter opens from the field: the first tool, or with none the first row of a reference. */
  protected readonly firstTool = computed(() =>
    this.searching() ? (this.panels()[0]?.tools[0]?.id ?? null) : null,
  );
  protected readonly firstHit = computed(() => {
    if (!this.searching() || this.firstTool() !== null) return null;
    const found = this.references()[0];
    const hit = found?.hits[0];
    return found && hit ? { tool: found.tool, hit } : null;
  });

  constructor() {
    afterNextRender(() => {
      if (typeof ResizeObserver === 'undefined') return;
      // A hidden page measures nothing: it keeps the columns it had when last shown.
      const observer = new ResizeObserver(([entry]) => {
        if (entry && entry.contentRect.width > 0) this.panelsWidth.set(entry.contentRect.width);
      });
      observer.observe(this.panelsBox().nativeElement);
      this.destroyRef.onDestroy(() => observer.disconnect());
    });

    // Ctrl+Shift+T may arrive before this home exists — from the notes — or while it shows.
    effect(() => {
      if (!this.store.searchWanted()) return;

      untracked(() => {
        this.store.searchTaken();
        afterNextRender(() => this.searchField().nativeElement.focus(), { injector: this.injector });
      });
    });
  }

  protected filterLabel(filter: ToolFilter): string {
    return filter === FAVOURITES ? 'tools.favourites.title' : `tools.categories.${filter}`;
  }

  protected onSearch(event: Event): void {
    this.store.setSearch((event.target as HTMLInputElement).value);
  }

  protected onKey(event: KeyboardEvent): void {
    if (event.target === this.searchField().nativeElement) this.onSearchKey(event);
    else this.onResultKey(event);
  }

  /** Enter opens the first result, the arrow goes down into them; Escape empties, then leaves. */
  private onSearchKey(event: KeyboardEvent): void {
    switch (event.key) {
      case 'Enter':
        if (!this.searching()) return;
        event.preventDefault();
        this.openFirst();
        return;
      case 'ArrowDown': {
        const first = this.results()[0];
        if (first === undefined) return;
        event.preventDefault();
        first.focus();
        return;
      }
      case 'Escape':
        event.preventDefault();
        if (this.store.search() === '') this.searchField().nativeElement.blur();
        else this.store.setSearch('');
        return;
    }
  }

  /** The arrows walk the results, tools then references; above the first is the field again. */
  private onResultKey(event: KeyboardEvent): void {
    const results = this.results();
    const at = results.indexOf(event.target as HTMLElement);
    if (at < 0) return;

    switch (event.key) {
      case 'ArrowDown':
        event.preventDefault();
        results[at + 1]?.focus();
        return;
      case 'ArrowUp':
        event.preventDefault();
        (results[at - 1] ?? this.searchField().nativeElement).focus();
        return;
      case 'Escape':
        if (!this.searching()) return;
        event.preventDefault();
        this.store.setSearch('');
        this.searchField().nativeElement.focus();
        return;
    }
  }

  /** On that row: its table whole, the row brought into view and lit. */
  protected openHit(tool: ToolDefinition, hit: ReferenceHit): void {
    this.sessions.slot(referenceQueryKey(tool.id), '').set('');
    this.store.open(tool.id, hit.key);
  }

  /** « n autres »: the reference searched for what was typed here — the same rows, all of them. */
  protected openAll(tool: ToolDefinition): void {
    this.sessions.slot(referenceQueryKey(tool.id), '').set(this.store.search().trim());
    this.store.open(tool.id);
  }

  private openFirst(): void {
    const tool = this.firstTool();
    if (tool !== null) return this.store.open(tool);
    const first = this.firstHit();
    if (first !== null) this.openHit(first.tool, first.hit);
  }

  /** In the order they are read: the panels' tools, then the references' rows. */
  private results(): HTMLElement[] {
    return [...this.host.nativeElement.querySelectorAll<HTMLElement>('[data-result]')];
  }

  private matches(tool: ToolDefinition, query: string): boolean {
    return toolMatches(query, [
      this.transloco.translate(`tools.${tool.id}.name`),
      this.transloco.translate(`tools.${tool.id}.description`),
      // « generateur » finds the three tools filed under « Générateurs ».
      this.transloco.translate(`tools.categories.${tool.category}`),
      ...tool.keywords,
    ]);
  }
}
