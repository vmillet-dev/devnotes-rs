import {
  ChangeDetectionStrategy,
  Component,
  ElementRef,
  afterRenderEffect,
  inject,
  signal,
  viewChild,
} from '@angular/core';
import { NgTemplateOutlet } from '@angular/common';
import { TranslocoPipe, TranslocoService } from '@jsverse/transloco';
import { HttpContents, HttpItem } from '@core/model/http.model';
import { AreaStore } from '@core/services/areas/area.store';
import { HttpCollectionsStore } from '@core/services/http/http-collections.store';
import { DropZone, KeyMove, RailRow, dropMove, keyMove } from '@core/services/http/http-tree';
import { SettingsStore } from '@core/services/settings/settings.store';
import { AreaSwitchComponent } from '@shared/controls/area-switch/area-switch.component';
import { HttpNodeAction, HttpNodeMenuComponent } from './http-node-menu/http-node-menu.component';

/** A name typed under a row: a collection at the head, a folder or a request inside one. */
interface Creating {
  readonly kind: 'collection' | 'folder' | 'request';
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

interface Drag {
  readonly row: RailRow;
  readonly pointerId: number;
  readonly x: number;
  readonly y: number;
  moved: boolean;
}

interface Drop {
  readonly id: string;
  readonly zone: DropZone;
}

/** Past this, a press on a row is a drag and no longer a click. */
const DRAG_THRESHOLD_PX = 4;

const MOVE_KEYS: Readonly<Record<string, KeyMove>> = {
  ArrowUp: 'up',
  ArrowDown: 'down',
  ArrowRight: 'in',
  ArrowLeft: 'out',
};

const item = (row: RailRow): HttpItem => ({ kind: row.kind, id: row.id });

/**
 * The collections, their folders and requests, in the library rail's frame. Moving is pointer
 * events, as every drag here — HTML5 drag and drop does not reach this WebView — doubled by
 * `Alt` and the arrows.
 */
@Component({
  selector: 'app-http-rail',
  imports: [AreaSwitchComponent, HttpNodeMenuComponent, NgTemplateOutlet, TranslocoPipe],
  templateUrl: './http-rail.component.html',
  styleUrl: './http-rail.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
  host: { '[style.width.px]': 'settings.libraryRailWidth()' },
})
export class HttpRailComponent {
  protected readonly areas = inject(AreaStore);
  protected readonly settings = inject(SettingsStore);
  protected readonly store = inject(HttpCollectionsStore);
  private readonly transloco = inject(TranslocoService);
  private readonly host = inject<ElementRef<HTMLElement>>(ElementRef);

  protected readonly creating = signal<Creating | null>(null);
  protected readonly renaming = signal<string | null>(null);
  protected readonly confirming = signal<Confirming | null>(null);
  protected readonly drop = signal<Drop | null>(null);
  protected readonly dragged = signal<string | null>(null);

  private drag: Drag | null = null;
  /** The click a drag ends with is not a click on the row it started from. */
  private swallowClick = false;

  private readonly field = viewChild<ElementRef<HTMLInputElement>>('field');

  constructor() {
    afterRenderEffect(() => {
      if (this.creating() !== null || this.renaming() !== null) {
        this.field()?.nativeElement.focus();
      }
    });
  }

  protected startCollection(): void {
    this.closeForms();
    this.creating.set({ kind: 'collection', collectionId: null, folderId: null, underId: null, depth: 0 });
  }

  protected activate(row: RailRow): void {
    if (this.swallowClick) {
      this.swallowClick = false;
      return;
    }
    if (row.kind === 'request') {
      this.store.open(row.id);
    } else {
      this.store.toggle(row.id);
    }
  }

  protected async onAction(row: RailRow, action: HttpNodeAction): Promise<void> {
    this.closeForms();
    switch (action) {
      case 'newRequest':
      case 'newFolder':
        this.creating.set({
          kind: action === 'newRequest' ? 'request' : 'folder',
          collectionId: row.collectionId,
          folderId: row.kind === 'folder' ? row.id : null,
          underId: row.id,
          depth: row.depth + 1,
        });
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

  protected async submitCreate(event: Event, name: string): Promise<void> {
    event.preventDefault();
    const creating = this.creating();
    const trimmed = name.trim();
    if (creating === null || trimmed === '') return;
    this.creating.set(null);
    if (creating.kind === 'collection') {
      await this.store.createCollection(trimmed);
    } else if (creating.kind === 'folder') {
      await this.store.createFolder(creating.collectionId ?? '', creating.folderId, trimmed);
    } else {
      await this.store.createRequest(creating.collectionId ?? '', creating.folderId, trimmed);
    }
  }

  protected async submitRename(event: Event, row: RailRow, name: string): Promise<void> {
    event.preventDefault();
    const trimmed = name.trim();
    this.renaming.set(null);
    if (trimmed !== '' && trimmed !== row.name) {
      await this.store.rename(item(row), trimmed);
    }
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
      if (answer !== null) {
        await this.store.apply(answer);
        this.focusRow(row.id);
      }
    } else if (event.key === 'F2') {
      event.preventDefault();
      this.closeForms();
      this.renaming.set(row.id);
    } else if (event.key === 'Delete') {
      event.preventDefault();
      await this.askDelete(row);
    }
  }

  protected startDrag(event: PointerEvent, row: RailRow): void {
    if (event.button !== 0) return;
    this.drag = { row, pointerId: event.pointerId, x: event.clientX, y: event.clientY, moved: false };
    (event.currentTarget as HTMLElement).setPointerCapture?.(event.pointerId);
  }

  protected moveDrag(event: PointerEvent): void {
    const drag = this.drag;
    if (drag?.pointerId !== event.pointerId) return;
    if (!drag.moved) {
      if (Math.hypot(event.clientX - drag.x, event.clientY - drag.y) < DRAG_THRESHOLD_PX) return;
      drag.moved = true;
      this.dragged.set(drag.row.id);
    }
    this.drop.set(this.dropAt(event.clientX, event.clientY, drag.row));
  }

  protected async endDrag(event: PointerEvent): Promise<void> {
    const drag = this.drag;
    if (drag?.pointerId !== event.pointerId) return;
    this.drag = null;
    const drop = this.drop();
    this.drop.set(null);
    this.dragged.set(null);
    if (!drag.moved) return;

    this.swallowClick = true;
    const target = drop === null ? undefined : this.store.rows().find((row) => row.id === drop.id);
    const move = target && drop ? dropMove(this.store.tree(), item(drag.row), item(target), drop.zone) : null;
    if (move !== null) await this.store.apply(move);
  }

  protected cancelDrag(): void {
    this.drag = null;
    this.drop.set(null);
    this.dragged.set(null);
  }

  protected noun(contents: HttpContents): string {
    return this.transloco.translate('http.rail.takesWith', {
      folders: contents.folders,
      requests: contents.requests,
    });
  }

  /** The row under the pointer and which third of it: a container takes what lands mid-row. */
  private dropAt(x: number, y: number, dragged: RailRow): Drop | null {
    const element = this.host.nativeElement.ownerDocument
      .elementFromPoint(x, y)
      ?.closest<HTMLElement>('[data-rail-row]');
    const id = element?.dataset['id'];
    const target = id === undefined ? undefined : this.store.rows().find((row) => row.id === id);
    if (!element || !target) return null;

    const box = element.getBoundingClientRect();
    const share = (y - box.top) / Math.max(box.height, 1);
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

  private async askDelete(row: RailRow): Promise<void> {
    const contents = await this.store.contents(item(row));
    if (contents !== null) this.confirming.set({ row, contents });
  }

  private focusRow(id: string): void {
    requestAnimationFrame(() =>
      this.host.nativeElement
        .querySelector<HTMLElement>(`[data-rail-row][data-id="${id}"] .node-name`)
        ?.focus(),
    );
  }
}
