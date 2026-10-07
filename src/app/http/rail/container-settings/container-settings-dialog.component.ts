import { ChangeDetectionStrategy, Component, OnInit, inject, input, output, signal } from '@angular/core';
import { TranslocoPipe } from '@jsverse/transloco';
import { HttpRepository } from '@core/data/http.repository';
import {
  ContainerSettingsDraft,
  DEFAULT_TRANSPORT,
  InheritedParts,
  KeyValueRow,
  RequestAuth,
  TransportSettings,
} from '@core/model/http.model';
import { ErrorNotifier } from '@core/services/errors/error-notifier.service';
import { RailRow } from '@core/services/http/http-tree';
import { DialogComponent } from '@shared/layout/dialog/dialog.component';
import { AuthEditorComponent } from '@http/ui/auth-editor/auth-editor.component';
import { KeyValueTableComponent } from '@http/ui/key-value-table/key-value-table.component';
import { TransportEditorComponent } from '@http/ui/transport-editor/transport-editor.component';

/** A collection's or a folder's auth and headers: what every request under it inherits. */
@Component({
  selector: 'app-container-settings-dialog',
  imports: [
    AuthEditorComponent,
    DialogComponent,
    KeyValueTableComponent,
    TranslocoPipe,
    TransportEditorComponent,
  ],
  templateUrl: './container-settings-dialog.component.html',
  styleUrl: './container-settings-dialog.component.scss',
  changeDetection: ChangeDetectionStrategy.OnPush,
})
export class ContainerSettingsDialogComponent implements OnInit {
  readonly row = input.required<RailRow>();
  readonly closed = output<void>();

  private readonly repository = inject(HttpRepository);
  private readonly errors = inject(ErrorNotifier);

  protected readonly settings = signal<ContainerSettingsDraft | null>(null);
  /** What a folder's « Héritée » takes: the folders and the collection above it. */
  protected readonly above = signal<InheritedParts | null>(null);
  protected readonly defaultTransport = DEFAULT_TRANSPORT;

  ngOnInit(): void {
    void this.load();
  }

  private async load(): Promise<void> {
    const row = this.row();
    const item = { kind: row.kind, id: row.id };
    this.settings.set(await this.errors.attempt('http.failed', () => this.repository.settings(item)));
    if (row.kind === 'folder') {
      this.above.set(
        await this.errors.attempt('http.failed', () =>
          this.repository.inherited(row.collectionId, row.folderId),
        ),
      );
    }
  }

  protected onAuth(auth: RequestAuth): void {
    this.settings.update((settings) => settings && { ...settings, auth });
  }

  protected onHeaders(headers: readonly KeyValueRow[]): void {
    this.settings.update((settings) => settings && { ...settings, headers });
  }

  protected onTransport(transport: TransportSettings): void {
    this.settings.update((settings) => settings && { ...settings, transport });
  }

  protected async save(): Promise<void> {
    const settings = this.settings();
    const row = this.row();
    if (settings === null) return;
    const saved = await this.errors.attempt('http.failed', async () => {
      await this.repository.saveSettings({ kind: row.kind, id: row.id }, settings);
      return true;
    });
    if (saved) this.closed.emit();
  }
}
