import { Provider } from '@angular/core';
import { CLIPBOARD_ADAPTER } from '@core/services/clipboard/clipboard.service';
import { DESKTOP_NOTIFICATION_ADAPTER } from '@core/services/notifications/desktop-notifier.service';
import { FILE_DIALOG_ADAPTER } from '@core/services/dialogs/file-dialog.service';
import { APP_WINDOW_ADAPTER } from '@core/services/window/app-window.service';
import { AttachmentsRepository } from '@core/data/attachments.repository';
import { BackupsRepository } from '@core/data/backups.repository';
import { BoardRepository } from '@core/data/board.repository';
import { FoldersRepository } from '@core/data/folders.repository';
import { NotesRepository } from '@core/data/notes.repository';
import { SpacesRepository } from '@core/data/spaces.repository';
import { TransferRepository } from '@core/data/transfer.repository';
import { VaultRepository } from '@core/data/vault.repository';
import { AppInfoService } from '@core/services/app-info/app-info.service';
import { Folder } from '@core/model/folder.model';
import { Note } from '@core/model/note.model';
import { Space } from '@core/model/space.model';
import { UpdaterService } from '@core/services/updates/updater.service';
import { FakeAppInfo } from './fake-app-info';
import { FakeAppWindow } from './fake-app-window';
import { FakeAttachmentsRepository } from './fake-attachments-repository';
import { FakeBackupsRepository } from './fake-backups-repository';
import { FakeBoardRepository } from './fake-board-repository';
import { FakeClipboard } from './fake-clipboard';
import { FakeDesktopNotifications } from './fake-desktop-notifications';
import { FakeFileDialog } from './fake-file-dialog';
import { FakeFoldersRepository } from './fake-folders-repository';
import { FakeNotesRepository } from './fake-notes-repository';
import { FakeSpacesRepository } from './fake-spaces-repository';
import { FakeTransferRepository } from './fake-transfer-repository';
import { FakeVaultRepository } from './fake-vault-repository';
import { FakeUpdater } from './fake-updater';
import { provideTranslocoTesting } from './provide-transloco-testing';

interface DataDoubles {
  readonly notes?: readonly Note[];
  readonly spaces?: readonly Space[];
  readonly folders?: readonly Folder[];
  /** Pass an existing fake to keep a handle on it (e.g. to set `failNext`). */
  readonly notesRepository?: FakeNotesRepository;
  readonly spacesRepository?: FakeSpacesRepository;
  readonly foldersRepository?: FakeFoldersRepository;
  readonly boardRepository?: FakeBoardRepository;
  readonly attachmentsRepository?: FakeAttachmentsRepository;
  readonly transferRepository?: FakeTransferRepository;
  readonly vaultRepository?: FakeVaultRepository;
  readonly backupsRepository?: FakeBackupsRepository;
  readonly updater?: FakeUpdater;
  readonly appInfo?: FakeAppInfo;
  readonly clipboard?: FakeClipboard;
  readonly desktopNotifications?: FakeDesktopNotifications;
  readonly fileDialog?: FakeFileDialog;
  readonly appWindow?: FakeAppWindow;
}

/** Bundled so a new data seam is not added to a dozen spec files one by one. */
export function provideAppTesting(doubles: DataDoubles = {}): Provider[] {
  return [
    {
      provide: BackupsRepository,
      useValue: doubles.backupsRepository ?? new FakeBackupsRepository(),
    },
    {
      provide: NotesRepository,
      useValue: doubles.notesRepository ?? new FakeNotesRepository(doubles.notes ?? []),
    },
    {
      provide: SpacesRepository,
      useValue: doubles.spacesRepository ?? new FakeSpacesRepository(doubles.spaces ?? []),
    },
    {
      provide: FoldersRepository,
      useValue: doubles.foldersRepository ?? new FakeFoldersRepository(doubles.folders ?? []),
    },
    {
      provide: BoardRepository,
      useValue: doubles.boardRepository ?? new FakeBoardRepository(),
    },
    {
      provide: AttachmentsRepository,
      useValue: doubles.attachmentsRepository ?? new FakeAttachmentsRepository(),
    },
    {
      provide: VaultRepository,
      useValue: doubles.vaultRepository ?? new FakeVaultRepository(),
    },
    {
      provide: TransferRepository,
      useValue: doubles.transferRepository ?? new FakeTransferRepository(),
    },
    // The shell hosts the update prompt and the about menu, so every spec reaching it
    // pulls the Tauri bridge, absent under jsdom.
    { provide: UpdaterService, useValue: doubles.updater ?? new FakeUpdater() },
    { provide: AppInfoService, useValue: doubles.appInfo ?? new FakeAppInfo() },
    { provide: CLIPBOARD_ADAPTER, useValue: doubles.clipboard ?? new FakeClipboard() },
    {
      provide: DESKTOP_NOTIFICATION_ADAPTER,
      useValue: doubles.desktopNotifications ?? new FakeDesktopNotifications(),
    },
    { provide: FILE_DIALOG_ADAPTER, useValue: doubles.fileDialog ?? new FakeFileDialog() },
    // Never the real one: `exit()` would take the test runner down with it.
    { provide: APP_WINDOW_ADAPTER, useValue: doubles.appWindow ?? new FakeAppWindow() },
    provideTranslocoTesting(),
  ];
}
