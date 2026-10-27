import {
  ChangeDetectionStrategy,
  Component,
  ElementRef,
  afterEveryRender,
  afterRenderEffect,
  computed,
  effect,
  inject,
  input,
  output,
  signal,
  untracked,
  viewChild,
} from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import type { FolderColour } from '@core/model/folder.model';
import { foldForSearch } from '@core/utils/fold.util';
import { FloatingPanelDirective } from '@shared/directives/floating-panel.directive';
import { MenuPanelDirective } from '@shared/directives/menu-panel.directive';
import { MenuTriggerDirective } from '@shared/directives/menu-trigger.directive';

/** Anything the menu can offer: a space, a folder, a language, a theme. */
export interface ChoiceOption {
  readonly id: string;
  readonly name: string;
  /** `name` is a translation key, translated in the template so it follows the language. */
  readonly nameIsKey?: boolean;
  /** Only a folder has one, and it is the same swatch the card's chip draws. */
  readonly colour?: FolderColour;
  /** Drawn faint after the name, and matched by the filter: a language's full name beside its label. */
  readonly hint?: string;
}

/** Past this many entries the panel opens on a filter field. */
export const FILTER_FROM = 9;

/** The "none" entry's key among the shown ones: no option id can be the empty string's twin. */
const NONE_KEY = '\u0000none';

let nextListId = 0;

/**
 * One choice among a list, as a menu rather than a native `<select>`, which brings the
 * operating system's border, arrow, focus ring and font. `naming: 'label'` makes it a command:
 * it shows what it does, and has no current value to announce.
 *
 * A long list opens as a filterable combobox instead (ARIA's select-only combobox with a
 * filter): the focus stays in the field and `aria-activedescendant` walks the list.
 */
@Component({
  selector: 'app-choice-menu',
  imports: [FloatingPanelDirective, MenuPanelDirective, TranslocoPipe],
  hostDirectives: [MenuTriggerDirective],
  // Used inside dialogs, whose own listener is the next one up for Escape: swallowed while the
  // panel is open, so one Escape does not close both.
  templateUrl: './choice-menu.component.html',
  styleUrl: './choice-menu.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ChoiceMenuComponent {
  readonly label = input.required<string>();
  /** Stable, unlike the translated `label`: what the tests address. */
  readonly kind = input.required<string>();
  readonly options = input.required<readonly ChoiceOption[]>();
  readonly currentId = input<string | null>(null);
  /** The entry that chooses none of them — a folder can be left, a space cannot. */
  readonly noneLabel = input<string | null>(null);
  /** `value` names what is chosen; `label` names what the menu does. */
  readonly naming = input<'value' | 'label'>('value');
  /** Replaces `choice-<kind>` on the trigger, where a scenario already addresses another one. */
  readonly triggerTestId = input<string | null>(null);
  /** Forces the filter field on or off; left unset, a list of `FILTER_FROM` entries gets one. */
  readonly filterable = input<boolean | null>(null);
  /** Said in the menu when it has no entry at all, rather than an empty panel. */
  readonly emptyLabel = input<string | null>(null);

  readonly chosen = output<string | null>();

  protected readonly menu = inject(MenuTriggerDirective);

  protected readonly current = computed(
    () => this.options().find((option) => option.id === this.currentId()) ?? null,
  );

  /** A command names what it does; a field names what is chosen, or its "none" entry. */
  protected readonly shown = computed(() => (this.naming() === 'label' ? null : this.current()));

  protected readonly fallbackText = computed(() =>
    this.naming() === 'label' ? this.label() : (this.noneLabel() ?? this.label()),
  );

  /** A command has no current entry to mark, and nothing to leave. */
  protected readonly showsNone = computed(() => this.naming() === 'value' && this.noneLabel() !== null);

  protected readonly filtering = computed(() => this.filterable() ?? this.options().length >= FILTER_FROM);

  protected readonly listId = `choice-list-${nextListId++}`;
  protected readonly noneKey = NONE_KEY;

  private readonly _query = signal('');
  protected readonly query = this._query.asReadonly();
  protected readonly folded = computed(() => foldForSearch(this._query().trim()));

  /**
   * What the list shows, read back from the rendered entries: an entry named by a key is
   * matched on its translation, which only the template's pipe knows — `shared/` injects no
   * service to translate it here.
   */
  private readonly _shownKeys = signal<readonly string[] | null>(null, {
    equal: (a, b) => a === b || (a !== null && b !== null && a.join('\u0000') === b.join('\u0000')),
  });
  private readonly _wantedKey = signal<string | null>(null);

  protected readonly activeKey = computed(() => {
    const keys = this._shownKeys() ?? [];
    const wanted = this._wantedKey();
    return wanted !== null && keys.includes(wanted) ? wanted : (keys[0] ?? null);
  });

  protected readonly nothingMatches = computed(() => this._shownKeys()?.length === 0);

  private readonly filterField = viewChild<ElementRef<HTMLInputElement>>('filterField');
  private readonly listbox = viewChild<ElementRef<HTMLElement>>('listbox');

  constructor() {
    this.menu.handleEscape((event) => this.onEscape(event));

    // Each opening starts afresh, on the current entry.
    effect(() => {
      const open = this.menu.open();
      untracked(() => {
        this._query.set('');
        this._shownKeys.set(null);
        this._wantedKey.set(open ? this.currentId() : null);
      });
    });

    // ⚠️ After `FloatingPanelDirective`'s `write` phase: until `showPopover` the panel is
    // hidden, and a hidden field refuses the focus.
    afterRenderEffect({ mixedReadWrite: () => this.filterField()?.nativeElement.focus() });

    afterEveryRender({
      read: () => {
        const list = this.listbox()?.nativeElement;
        if (!list) return;
        this._shownKeys.set(
          Array.from(
            list.querySelectorAll<HTMLElement>('[data-choice-key]'),
            (entry) => entry.dataset['choiceKey']!,
          ),
        );
        list.querySelector('.active')?.scrollIntoView?.({ block: 'nearest' });
      },
    });
  }

  protected matches(...texts: (string | null | undefined)[]): boolean {
    const wanted = this.folded();
    return texts.some((text) => text != null && foldForSearch(text).includes(wanted));
  }

  protected entryId(key: string | null): string | null {
    if (key === null) return null;
    if (key === NONE_KEY) return `${this.listId}-none`;
    const index = this.options().findIndex((option) => option.id === key);
    return index < 0 ? null : `${this.listId}-${index}`;
  }

  protected onQuery(value: string): void {
    this._query.set(value);
    this._wantedKey.set(null);
  }

  protected onFilterKeydown(event: KeyboardEvent): void {
    const keys = this._shownKeys() ?? [];
    const at = keys.indexOf(this.activeKey() ?? '');
    const move = (index: number): void => {
      event.preventDefault();
      if (keys.length > 0) this._wantedKey.set(keys[(index + keys.length) % keys.length] ?? null);
    };

    switch (event.key) {
      case 'ArrowDown':
        move(at + 1);
        break;
      case 'ArrowUp':
        move(at - 1);
        break;
      case 'Enter': {
        event.preventDefault();
        const key = this.activeKey();
        if (key !== null) this.pick(key === NONE_KEY ? null : key);
        break;
      }
    }
  }

  /** Swallowed only while open: closed, the key belongs to the dialog. A filter is cleared first. */
  private onEscape(event: Event): void {
    if (!this.menu.open()) return;

    event.stopPropagation();
    if (this.filtering() && this._query() !== '') {
      this.onQuery('');
      return;
    }
    this.menu.close();
  }

  protected pick(id: string | null): void {
    this.menu.close();
    // Asking for where it already is is asking for nothing — but a command has no "already".
    if (this.naming() === 'value' && id === this.currentId()) return;

    this.chosen.emit(id);
  }
}
