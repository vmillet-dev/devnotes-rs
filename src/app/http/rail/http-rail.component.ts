import {
  ChangeDetectionStrategy,
  Component,
  DestroyRef,
  ElementRef,
  afterRenderEffect,
  computed,
  inject,
  signal,
  viewChild,
} from '@angular/core';
import { NgTemplateOutlet } from '@angular/common';
import { TranslocoPipe, TranslocoService } from '@jsverse/transloco';
import { HttpContents, HttpItem, RequestKind, shortBadge } from '@core/model/http.model';
import { AreaStore } from '@core/services/areas/area.store';
import { HttpCollectionsStore } from '@core/services/http/http-collections.store';
import { HttpRunnerStore } from '@core/services/http/http-runner.store';
import { HttpEnvironmentsStore } from '@core/services/http/http-environments.store';
import { HttpHistoryStore } from '@core/services/http/http-history.store';
import { HttpWorkspaceStore } from '@core/services/http/http-workspace.store';
import { HttpTabsStore } from '@core/services/http/http-tabs.store';
import {
  DropZone,
  KeyMove,
  RailRow,
  dropMove,
  keyMove,
  landing,
  refusal,
  requestsUnder,
} from '@core/services/http/http-tree';
import { RAIL_WIDTH } from '@core/services/settings/app-settings.model';
import { SettingsStore } from '@core/services/settings/settings.store';
import { AreaSwitchComponent } from '@shared/controls/area-switch/area-switch.component';
import { IconComponent } from '@shared/icon/icon.component';
import { RailFrameComponent } from '@shared/layout/rail-frame/rail-frame.component';
import { RailMarks } from '@shared/rail/rail-marks';
import { RowDrag, railRowAt, shareOf } from '@shared/rail/row-drag';
import { ContainerSettingsDialogComponent } from './container-settings/container-settings-dialog.component';
import { CookieJarDialogComponent } from './cookie-jar/cookie-jar-dialog.component';
import { HttpNodeAction, HttpNodeMenuComponent } from './http-node-menu/http-node-menu.component';

/** A name typed under a row: a collection at the head, a folder or a request inside one. */
interface Creating {
  readonly kind: 'collection' | 'folder' | 'request';
  /** What a request is created as, chosen in the menu before its name is typed; `null` otherwise. */
  readonly requestKind: RequestKind | null;
  readonly collectionId: string | null;
  readonly folderId: string | null;
  /** The row it is typed under, `null` at the head of the rail. */
  readonly underId: string | null;
  readonly depth: number;
}

interface Confirming {
  readonly row: RailRow;
  readonly contents: HttpContents;
}

interface Drop {
  readonly id: string;
  readonly zone: DropZone;
}

/** From this width the head has room for « ＋ Collection » beside its title; under it, an icon. */
const WORDED_FROM_PX = 216;

const MOVE_KEYS: Readonly<Record<string, KeyMove>> = {
  ArrowUp: 'up',
  ArrowDown: 'down',
  ArrowRight: 'in',
  ArrowLeft: 'out',
};

const item = (row: RailRow): HttpItem => ({ kind: row.kind, id: row.id });

/** The kind each « nouvelle requête » of a row's menu creates. */
const NEW_KINDS: Partial<Record<HttpNodeAction, RequestKind>> = {
  newHttp: 'http',
  newGraphql: 'graphql',
  newWebsocket: 'websocket',
};

/**
 * The collections, their folders and requests, in the library rail's frame. Moving is pointer
 * events, as every drag here — HTML5 drag and drop does not reach this WebView — doubled by
 * `Alt` and the arrows.
 */
@Component({
  selector: 'app-http-rail',
  imports: [
    AreaSwitchComponent,
    ContainerSettingsDialogComponent,
    CookieJarDialogComponent,
    HttpNodeMenuComponent,
    IconComponent,
    NgTemplateOutlet,
    RailFrameComponent,
    TranslocoPipe,
  ],
  templateUrl: './http-rail.component.html',
  styleUrl: './http-rail.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class HttpRailComponent {
  protected readonly areas = inject(AreaStore);
  protected readonly settings = inject(SettingsStore);
  protected readonly store = inject(HttpCollectionsStore);
  private readonly runner = inject(HttpRunnerStore);
  protected readonly tabs = inject(HttpTabsStore);
  protected readonly history = inject(HttpHistoryStore);
  protected readonly environments = inject(HttpEnvironmentsStore);
  private readonly workspace = inject(HttpWorkspaceStore);
  private readonly transloco = inject(TranslocoService);
  private readonly host = inject<ElementRef<HTMLElement>>(ElementRef);

  protected readonly bounds = RAIL_WIDTH;
  protected readonly short = shortBadge;

  protected readonly creating = signal<Creating | null>(null);
  protected readonly renaming = signal<string | null>(null);
  protected readonly confirming = signal<Confirming | null>(null);
  protected readonly cookiesOpen = signal(false);
  /** The tabs' « + » begins a request a few pixels away: this one says what it makes. */
  protected readonly worded = computed(() => this.settings.libraryRailWidth() >= WORDED_FROM_PX);
  protected readonly drag = new RowDrag<RailRow, Drop>((x, y, row) => this.dropAt(x, y, row));
  protected readonly marks = new RailMarks('http.rail.moved');

  private readonly field = viewChild<ElementRef<HTMLInputElement>>('field');

  constructor() {
    inject(DestroyRef).onDestroy(() => this.marks.dispose());
    afterRenderEffect(() => {
      if (this.creating() !== null || this.renaming() !== null) {
        this.field()?.nativeElement.focus();
      }
    });
  }

  protected startCollection(): void {
    this.closeForms();
    this.creating.set({
      kind: 'collection',
      requestKind: null,
      collectionId: null,
      folderId: null,
      underId: null,
      depth: 0,
    });
  }

  protected activate(row: RailRow): void {
    if (this.drag.swallowsClick()) return;
    if (row.kind === 'request') {
      this.workspace.show('requests');
      void this.tabs.open(row.id);
    } else {
      this.store.toggle(row.id);
    }
  }

  protected async onAction(row: RailRow, action: HttpNodeAction): Promise<void> {
    this.closeForms();
    switch (action) {
      case 'newHttp':
      case 'newGraphql':
      case 'newWebsocket':
      case 'newFolder':
        this.creating.set({
          kind: action === 'newFolder' ? 'folder' : 'request',
          requestKind: NEW_KINDS[action] ?? null,
          collectionId: row.collectionId,
          folderId: row.kind === 'folder' ? row.id : null,
          underId: row.id,
          depth: row.depth + 1,
        });
        return;
      case 'run':
        this.runner.open(row);
        return;
      case 'settings':
        this.store.openSettings(row);
        return;
      case 'rename':
        this.renaming.set(row.id);
        return;
      case 'duplicate':
        await this.store.duplicate(
          item(row),
          this.transloco.translate('http.rail.copyName', { name: row.name }),
        );
        return;
      case 'delete':
        await this.askDelete(row);
        return;
    }
  }

  protected async submitCreate(event: Event, form: Creating, name: string): Promise<void> {
    event.preventDefault();
    if (name.trim() !== '') await this.settleCreate(form, name);
  }

  protected async submitRename(event: Event, row: RailRow, name: string): Promise<void> {
    event.preventDefault();
    await this.settleRename(row, name);
  }

  /**
   * A form left for anything outside it keeps what was typed, as a file explorer does; left
   * empty, it closes. Escape is the way to cancel.
   */
  protected leaveCreate(event: FocusEvent, form: Creating, name: string): void {
    if (this.leaves(event)) void this.settleCreate(form, name);
  }

  protected leaveRename(event: FocusEvent, row: RailRow, name: string): void {
    if (this.leaves(event)) void this.settleRename(row, name);
  }

  protected async confirmDelete(): Promise<void> {
    const confirming = this.confirming();
    if (confirming === null) return;
    this.confirming.set(null);
    await this.store.delete(item(confirming.row));
  }

  protected closeForms(): void {
    this.creating.set(null);
    this.renaming.set(null);
    this.confirming.set(null);
  }

  protected async onRowKeydown(event: KeyboardEvent, row: RailRow): Promise<void> {
    const move = MOVE_KEYS[event.key];
    if (event.altKey && !event.ctrlKey && !event.metaKey && move !== undefined) {
      event.preventDefault();
      const answer = keyMove(this.store.tree(), item(row), move);
      if (answer === null) {
        this.marks.say(`refused.${refusal(item(row), move)}`, { name: row.name });
        return;
      }
      await this.store.apply(answer);
      const landed = landing(this.store.tree(), item(row), move);
      if (landed !== null) {
        this.marks.mark({ id: row.id, parentId: landed.parentId });
        this.marks.say(landed.where, { name: row.name, other: landed.other });
      }
      this.focusRow(row.id);
    } else if (event.key === 'F2') {
      event.preventDefault();
      this.closeForms();
      this.renaming.set(row.id);
    } else if (event.key === 'Delete') {
      event.preventDefault();
      await this.askDelete(row);
    }
  }

  protected async endDrag(event: PointerEvent): Promise<void> {
    const ended = this.drag.end(event);
    const drop = ended?.target;
    if (!ended || !drop) return;

    const target = this.store.rows().find((row) => row.id === drop.id);
    const move = target ? dropMove(this.store.tree(), item(ended.row), item(target), drop.zone) : null;
    if (move !== null) await this.store.apply(move);
  }

  /**
   * The open tabs a deletion would close with changes in them. Read as the confirmation is
   * drawn, not when it was asked: a tab saved in between is no longer a loss.
   */
  protected unsaved(row: RailRow): string[] {
    return this.tabs.unsavedAmong(requestsUnder(this.store.tree(), item(row)));
  }

  /** Says only what is there: « aucun dossier » is noise in a sentence about loss. */
  protected noun(contents: HttpContents): string {
    const which = contents.folders === 0 ? 'requests' : contents.requests === 0 ? 'folders' : 'both';
    return this.transloco.translate(`http.rail.takes.${which}`, {
      folders: contents.folders,
      requests: contents.requests,
    });
  }

  /** The row under the pointer and which third of it: a container takes what lands mid-row. */
  private dropAt(x: number, y: number, dragged: RailRow): Drop | null {
    const element = railRowAt(this.host.nativeElement.ownerDocument, x, y);
    const id = element?.dataset['id'];
    const target = id === undefined ? undefined : this.store.rows().find((row) => row.id === id);
    if (!element || !target) return null;

    const share = shareOf(element, y);
    let zone: DropZone;
    if (target.kind === 'request') {
      zone = share < 0.5 ? 'before' : 'after';
    } else if (target.kind === 'collection' && dragged.kind !== 'collection') {
      zone = 'inside';
    } else {
      zone = share < 0.25 ? 'before' : share > 0.75 ? 'after' : 'inside';
    }
    return dropMove(this.store.tree(), item(dragged), item(target), zone) === null
      ? null
      : { id: target.id, zone };
  }

  /** From the field to its own button is not leaving: the press that follows is the submit. */
  private leaves(event: FocusEvent): boolean {
    const form = event.currentTarget as HTMLElement;
    return !(event.relatedTarget instanceof Node && form.contains(event.relatedTarget));
  }

  /**
   * ⚠️ The form is taken first, before anything is awaited: Enter, the button and the field
   * left can each arrive for the same name — a removed field loses its focus — and whichever
   * comes second finds its form gone, or another in its place, and writes nothing.
   */
  private async settleCreate(creating: Creating, name: string): Promise<void> {
    if (this.creating() !== creating) return;
    this.creating.set(null);
    const trimmed = name.trim();
    if (trimmed === '') return;
    if (creating.kind === 'collection') {
      await this.store.createCollection(trimmed);
    } else if (creating.kind === 'folder') {
      await this.store.createFolder(creating.collectionId ?? '', creating.folderId, trimmed);
    } else {
      const id = await this.store.createRequest(
        creating.collectionId ?? '',
        creating.folderId,
        trimmed,
        creating.requestKind ?? 'http',
      );
      if (id !== null) {
        this.workspace.show('requests');
        await this.tabs.open(id);
      }
    }
  }

  private async settleRename(row: RailRow, name: string): Promise<void> {
    if (this.renaming() !== row.id) return;
    this.renaming.set(null);
    const trimmed = name.trim();
    if (trimmed !== '' && trimmed !== row.name) {
      await this.store.rename(item(row), trimmed);
    }
  }

  private async askDelete(row: RailRow): Promise<void> {
    const contents = await this.store.contents(item(row));
    if (contents !== null) this.confirming.set({ row, contents });
  }

  private focusRow(id: string): void {
    requestAnimationFrame(() => {
      const name = this.host.nativeElement.querySelector<HTMLElement>(
        `[data-rail-row][data-id="${id}"] .node-name`,
      );
      name?.focus();
      // Absent from jsdom, and from nothing else this runs on.
      name?.scrollIntoView?.({ block: 'nearest' });
    });
  }
}
