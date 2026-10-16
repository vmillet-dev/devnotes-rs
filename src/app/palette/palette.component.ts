import {
  ChangeDetectionStrategy,
  Component,
  DestroyRef,
  computed,
  inject,
  linkedSignal,
} from '@angular/core';
import { toSignal } from '@angular/core/rxjs-interop';
import { TranslocoPipe, TranslocoService } from '@jsverse/transloco';
import { AppEventsService } from '@core/ipc/app-events.service';
import { RequestHit, requestBadge } from '@core/model/http.model';
import { Note } from '@core/model/note.model';
import { AreaStore } from '@core/services/areas/area.store';
import { HelpStore } from '@core/services/help/help.store';
import { HttpTabsStore } from '@core/services/http/http-tabs.store';
import { PreferencesDialogStore } from '@core/services/settings/preferences-dialog.store';
import { ShortcutBindingsStore } from '@core/services/shortcuts/shortcut-bindings.store';
import { acceleratorKeys } from '@core/services/shortcuts/shortcut.model';
import { ToolDefinition, favouritesAmong, toolMatches } from '@core/services/tools/tool.model';
import { ToolsStore } from '@core/services/tools/tools.store';
import { FoldersStore } from '@core/state/folders.store';
import { NotesStore } from '@core/state/notes.store';
import { PALETTE_KINDS, PaletteChip, PaletteKind, PaletteStore } from '@core/state/palette.store';
import { PlaceholderFillStore } from '@core/state/placeholder-fill.store';
import { SpacesStore } from '@core/state/spaces.store';
import { foldForSearch } from '@core/utils/fold.util';
import { PlaceholderFormComponent } from '@app/placeholder-form/placeholder-form.component';
import { KindBadgeComponent } from '@shared/badges/kind-badge/kind-badge.component';
import { IconComponent } from '@shared/icon/icon.component';
import { DialogComponent } from '@shared/layout/dialog/dialog.component';
import { TOOLS, TOOL_CATEGORY_ICONS } from '@tools/catalogue/catalogue';
import { PALETTE_COMMANDS, PaletteCommand, commandId } from './palette-commands';
import {
  PaletteGroup,
  PaletteOption,
  chipGroup,
  commandGroup,
  recentGroups,
  searchGroups,
} from './palette-options';

/** The words `@` is read by, in either language whatever the one on screen, accents aside. */
const SCOPE_WORDS: Readonly<Record<PaletteKind, readonly string[]>> = {
  notes: ['notes', 'snippets'],
  requests: ['requetes', 'requests', 'http'],
  tools: ['outils', 'tools'],
  spaces: ['espaces', 'spaces'],
};

const MAX_TAGS = 8;

/** A key and what it does, for the line under the list. */
interface Hint {
  readonly keys: readonly string[];
  readonly labelKey: string;
}

interface Part {
  readonly text: string;
  readonly hit: boolean;
}

/**
 * The one palette, drawn over whichever area is on screen. `aria-activedescendant` and not a
 * moved focus, which would lose what is being typed; the rows are out of the tab order so
 * there is one current row. What Enter does to each kind is decided here: the stores it
 * reaches would make a cycle of `PaletteStore`.
 */
@Component({
  selector: 'app-palette',
  imports: [DialogComponent, IconComponent, KindBadgeComponent, PlaceholderFormComponent, TranslocoPipe],
  templateUrl: './palette.component.html',
  styleUrl: './palette.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class PaletteComponent {
  protected readonly store = inject(PaletteStore);
  protected readonly fill = inject(PlaceholderFillStore);
  private readonly notes = inject(NotesStore);
  private readonly areas = inject(AreaStore);
  private readonly tabs = inject(HttpTabsStore);
  private readonly tools = inject(ToolsStore);
  private readonly spaces = inject(SpacesStore);
  private readonly folders = inject(FoldersStore);
  private readonly help = inject(HelpStore);
  private readonly preferences = inject(PreferencesDialogStore);
  private readonly bindings = inject(ShortcutBindingsStore);
  private readonly transloco = inject(TranslocoService);

  private readonly lang = toSignal(this.transloco.langChanges$, {
    initialValue: this.transloco.getActiveLang(),
  });

  protected readonly categoryIcons = TOOL_CATEGORY_ICONS;
  protected readonly commandId = commandId;

  /** The tools as they read on screen: their search is the front's, as the tools' home's is. */
  private readonly toolWords = computed(() => {
    this.lang();
    return TOOLS.map((tool) => ({
      tool,
      words: [
        this.translate(`tools.${tool.id}.name`),
        this.translate(`tools.${tool.id}.description`),
        this.translate(`tools.categories.${tool.category}`),
        ...tool.keywords,
      ],
    }));
  });

  private readonly spaceNames = computed(
    () => new Map(this.spaces.spaces().map((space) => [space.id, space.name] as const)),
  );

  private readonly foundTools = computed<readonly ToolDefinition[]>(() => {
    if (this.store.recent())
      return favouritesAmong(
        TOOLS,
        this.tools.recents().map((recent) => recent.id),
      );
    const mode = this.store.mode();
    if (mode.kind !== 'search' || !this.store.searched().has('tools')) return [];
    return this.toolWords()
      .filter(({ words }) => toolMatches(mode.term, words))
      .map(({ tool }) => tool);
  });

  private readonly foundSpaces = computed(() => {
    const mode = this.store.mode();
    if (this.store.recent() || mode.kind !== 'search' || !this.store.searched().has('spaces')) return [];
    const wanted = foldForSearch(mode.term);
    return this.spaces.spaces().filter((space) => foldForSearch(space.name).includes(wanted));
  });

  protected readonly groups = computed<readonly PaletteGroup[]>(() => {
    const mode = this.store.mode();
    switch (mode.kind) {
      case 'scopes':
        return this.scopeGroups(foldForSearch(mode.word));
      case 'tags':
        return this.tagGroups(foldForSearch(mode.word));
      case 'commands':
        return [commandGroup(this.commandsMatching(foldForSearch(mode.term)))].filter(
          (group) => group.options.length > 0,
        );
      case 'search': {
        const found = {
          notes: this.store.notes(),
          requests: this.store.requests(),
          tools: this.foundTools(),
          spaces: this.foundSpaces(),
        };
        return this.store.recent()
          ? recentGroups(found)
          : searchGroups(found, this.store.searched(), mode.term);
      }
    }
  });

  protected readonly options = computed(() => this.groups().flatMap((group) => group.options));

  /** Back to the first whenever the list changes: what was under it may have gone. */
  private readonly _highlighted = linkedSignal({ source: this.options, computation: () => 0 });

  protected readonly highlighted = computed(() =>
    Math.min(this._highlighted(), Math.max(0, this.options().length - 1)),
  );

  protected readonly current = computed<PaletteOption | null>(
    () => this.options()[this.highlighted()] ?? null,
  );

  protected readonly hints = computed<readonly Hint[]>(() => {
    const hints: Hint[] = [{ keys: ['↑', '↓'], labelKey: 'palette.keys.navigate' }];
    const current = this.current();
    switch (current?.kind) {
      case 'note':
        hints.push(
          { keys: ['↵'], labelKey: 'palette.keys.open' },
          {
            keys: ['Ctrl', 'C'],
            labelKey: this.store.fromElsewhere() ? 'palette.keys.copyBack' : 'palette.keys.copy',
          },
        );
        break;
      case 'chip':
        hints.push({ keys: ['Tab', '↵'], labelKey: 'palette.keys.choose' });
        break;
      case 'command':
        hints.push({ keys: ['↵'], labelKey: 'palette.keys.run' });
        break;
      case 'create':
        hints.push({ keys: ['↵'], labelKey: 'palette.keys.create' });
        break;
      case 'request':
      case 'tool':
      case 'space':
        hints.push({ keys: ['↵'], labelKey: 'palette.keys.open' });
        break;
      case undefined:
        break;
    }
    if (this.store.chips().length > 0 && this.store.text() === '') {
      hints.push({ keys: ['⌫'], labelKey: 'palette.keys.unchip' });
    }
    hints.push({ keys: ['Esc'], labelKey: 'palette.keys.close' });
    return hints;
  });

  /** Said in place of a list: what was looked for, so a typo shows. */
  protected readonly empty = computed<{ key: string; params: Record<string, string> } | null>(() => {
    if (this.options().length > 0) return null;
    const mode = this.store.mode();
    switch (mode.kind) {
      case 'scopes':
        return { key: 'palette.empty.scopes', params: { word: mode.word } };
      case 'tags':
        return { key: 'palette.empty.tags', params: { word: mode.word } };
      case 'commands':
        return { key: 'palette.empty.commands', params: { term: mode.term } };
      case 'search':
        return mode.term === ''
          ? { key: 'palette.empty.nothing', params: {} }
          : { key: 'palette.empty.search', params: { term: mode.term } };
    }
  });

  constructor() {
    const events = inject(AppEventsService);
    inject(DestroyRef).onDestroy(
      events.on((action) => {
        if (action === 'palette' || action === 'palette-here') void this.store.open(action === 'palette');
      }),
    );
  }

  protected optionId(index: number): string {
    return `palette-option-${index}`;
  }

  protected indexOf(option: PaletteOption): number {
    return this.options().indexOf(option);
  }

  protected chipLabel(chip: PaletteChip): string {
    switch (chip.kind) {
      case 'scope':
        return `@${this.translate(`palette.scopes.${chip.scope}`)}`;
      case 'space':
        return `@${chip.name}`;
      case 'tag':
        return `#${chip.tag}`;
    }
  }

  protected spaceOf(note: Note): string {
    return this.spaceNames().get(note.spaceId) ?? '';
  }

  /** The first line that says something: a todo list's first item, a Note's first line. */
  protected lineOf(note: Note): string {
    const text = note.kind === 'checklist' ? (note.items[0]?.text ?? '') : note.content;
    return (
      text
        .split('\n')
        .map((line) => line.trim())
        .find((line) => line !== '') ?? ''
    );
  }

  protected badgeOf(request: RequestHit): string {
    return requestBadge(request);
  }

  /** `{{baseUrl}}` picked out, as the request's own field draws it. */
  protected urlParts(url: string): readonly Part[] {
    return url
      .split(/(\{\{[A-Za-z0-9_-]+\}\})/)
      .filter((text) => text !== '')
      .map((text) => ({ text, hit: text.startsWith('{{') }));
  }

  /** What was typed, lit in the title, when folding keeps the title's length. */
  protected titleParts(title: string): readonly Part[] {
    const mode = this.store.mode();
    const term = mode.kind === 'search' || mode.kind === 'commands' ? mode.term : '';
    const folded = foldForSearch(title);
    const at = term === '' || folded.length !== title.length ? -1 : folded.indexOf(foldForSearch(term));
    if (at < 0) return [{ text: title, hit: false }];
    return [
      { text: title.slice(0, at), hit: false },
      { text: title.slice(at, at + term.length), hit: true },
      { text: title.slice(at + term.length), hit: false },
    ].filter((part) => part.text !== '');
  }

  protected keysOf(command: PaletteCommand): readonly string[] {
    switch (command.kind) {
      case 'create':
        return acceleratorKeys(this.bindings.binding(command.action));
      case 'area':
        return command.keys;
      case 'request':
      case 'folder':
      case 'preferences':
      case 'shortcuts':
        return [];
    }
  }

  protected onText(event: Event): void {
    this.store.setText((event.target as HTMLInputElement).value);
  }

  protected highlight(index: number): void {
    this._highlighted.set(index);
  }

  /** Stops at both ends: wrapping around would lose track of where one is. */
  private move(step: number): void {
    const last = this.options().length - 1;
    this._highlighted.set(Math.max(0, Math.min(last, this.highlighted() + step)));
  }

  protected onKeydown(event: KeyboardEvent): void {
    // `Ctrl+C` and not a bare `c`, which the field would simply type. A selection in the
    // field keeps its own meaning.
    if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'c') {
      const field = event.target;
      const selecting = field instanceof HTMLInputElement && field.selectionStart !== field.selectionEnd;
      const current = this.current();
      if (selecting || current?.kind !== 'note') return;

      event.preventDefault();
      void this.store.copy(current.note);
      return;
    }

    switch (event.key) {
      case 'ArrowDown':
        event.preventDefault();
        this.move(1);
        break;
      case 'ArrowUp':
        event.preventDefault();
        this.move(-1);
        break;
      // The focus never leaves the field, so Tab has nowhere useful to go: it chooses a chip
      // offered after `@` or `#`, and is a slower arrow everywhere else.
      case 'Tab': {
        event.preventDefault();
        const current = this.current();
        if (!event.shiftKey && current?.kind === 'chip') {
          this.choose(current);
        } else {
          this.move(event.shiftKey ? -1 : 1);
        }
        break;
      }
      case 'Enter': {
        event.preventDefault();
        const current = this.current();
        if (current) this.choose(current);
        break;
      }
      case 'Backspace':
        if (this.store.text() === '' && this.store.chips().length > 0) {
          event.preventDefault();
          this.store.removeChip(this.store.chips().length - 1);
        }
        break;
    }
  }

  protected copy(note: Note): void {
    void this.store.copy(note);
  }

  /** Opens, as a click on the row does and as Enter does everywhere else in the application. */
  protected choose(option: PaletteOption): void {
    if (option.kind === 'chip') {
      this.store.addChip(option.chip);
      return;
    }

    this.store.close();
    switch (option.kind) {
      case 'note':
        this.areas.show('notes');
        void this.notes.openNote(option.note.id);
        break;
      case 'request':
        void this.openRequest(option.request.id);
        break;
      case 'tool':
        this.tools.open(option.tool.id);
        this.areas.show('tools');
        break;
      case 'space':
        this.areas.show('notes');
        this.spaces.selectSpace(option.space.id);
        break;
      case 'command':
        this.run(option.command);
        break;
      case 'create':
        this.areas.show('notes');
        void this.notes.createTitled(option.title);
        break;
    }
  }

  /** The tabs left open last time come back first: opened after, the request is the one in front. */
  private async openRequest(id: string): Promise<void> {
    this.areas.show('http');
    await this.tabs.restore();
    await this.tabs.open(id);
  }

  private run(command: PaletteCommand): void {
    switch (command.kind) {
      case 'create':
        this.areas.show('notes');
        this.notes.createNote(command.note);
        break;
      case 'request':
        this.areas.show('http');
        void this.tabs.restore().then(() => this.tabs.newRequest());
        break;
      case 'folder':
        this.areas.show('notes');
        this.folders.askName();
        break;
      case 'area':
        this.areas.show(command.area);
        break;
      case 'preferences':
        this.preferences.open();
        break;
      case 'shortcuts':
        this.help.show('shortcuts');
        break;
    }
  }

  private scopeGroups(word: string): readonly PaletteGroup[] {
    const chosen = new Set(this.store.chips().flatMap((chip) => (chip.kind === 'scope' ? [chip.scope] : [])));
    const scopes = PALETTE_KINDS.filter(
      (kind) => !chosen.has(kind) && SCOPE_WORDS[kind].some((candidate) => candidate.startsWith(word)),
    );
    const spaces = this.spaces.spaces().filter((space) => foldForSearch(space.name).startsWith(word));
    return [
      chipGroup(
        'scopes',
        scopes.map((scope) => [{ kind: 'scope', scope }, null] as const),
      ),
      chipGroup(
        'spaceNotes',
        spaces.map((space) => [{ kind: 'space', id: space.id, name: space.name }, null] as const),
      ),
    ].filter((group) => group.options.length > 0);
  }

  private tagGroups(word: string): readonly PaletteGroup[] {
    const chosen = new Set(this.store.chips().flatMap((chip) => (chip.kind === 'tag' ? [chip.tag] : [])));
    // The most used first: the tags are read in their name's order.
    const tags = this.store
      .tags()
      .filter((usage) => !chosen.has(usage.tag) && foldForSearch(usage.tag).startsWith(word))
      .sort((a, b) => b.noteCount - a.noteCount)
      .slice(0, MAX_TAGS);
    if (tags.length === 0) return [];
    return [
      chipGroup(
        'tags',
        tags.map((usage) => [{ kind: 'tag', tag: usage.tag }, usage.noteCount] as const),
      ),
    ];
  }

  /** « Nouveau dossier » needs a space to put it in: across every space it is not offered. */
  private commandsMatching(term: string): readonly PaletteCommand[] {
    this.lang();
    return PALETTE_COMMANDS.filter(
      (command) =>
        (command.kind !== 'folder' || this.spaces.activeSpaceId() !== null) &&
        foldForSearch(this.translate(command.labelKey)).includes(term),
    );
  }

  private translate(key: string): string {
    return this.transloco.translate(key);
  }
}
