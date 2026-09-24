import { TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';
import { FakeFileDialog } from '@testing/fake-file-dialog';
import { FILE_DIALOG_ADAPTER, FileDialogService } from './file-dialog.service';

describe('FileDialogService', () => {
  let adapter: FakeFileDialog;
  let service: FileDialogService;

  beforeEach(() => {
    TestBed.resetTestingModule();
    adapter = new FakeFileDialog();
    TestBed.configureTestingModule({
      providers: [{ provide: FILE_DIALOG_ADAPTER, useValue: adapter }],
    });
    service = TestBed.inject(FileDialogService);
  });

  it('hands back the chosen path', async () => {
    adapter.openPath = 'C:/notes/devnotes.json';

    expect(await service.pickBundle()).toBe('C:/notes/devnotes.json');
  });

  it('reduces the plugin union to a single path', async () => {
    adapter.openPath = ['C:/first.png', 'C:/second.png'];

    expect(await service.pickAttachment()).toBe('C:/first.png');
  });

  it('reports a cancelled dialog as no choice', async () => {
    adapter.openPath = null;

    expect(await service.pickBundle()).toBeNull();
  });

  it('treats an unavailable plugin as a cancelled dialog', async () => {
    adapter.throwOnOpen = new Error('no bridge');

    expect(await service.pickBundle()).toBeNull();
  });

  /** `json` stays on the way in: an export written before the archive existed is
   *  still importable, and the picker has to let the user reach it. */
  it('filters the bundle picker on the exchange format, old one included', async () => {
    await service.pickBundle();

    expect(adapter.openCalls[0].filters?.[0].extensions).toEqual(['devnotes', 'json']);
  });

  it('offers only the archive when saving', async () => {
    await service.chooseBundleDestination('library.devnotes');

    expect(adapter.saveCalls[0].filters?.[0].extensions).toEqual(['devnotes']);
  });

  it('leaves the attachment picker unfiltered', async () => {
    await service.pickAttachment();

    expect(adapter.openCalls[0].filters).toBeUndefined();
  });

  it('proposes the given file name when saving', async () => {
    adapter.savePath = 'C:/out/devnotes-2026-08-27.json';

    const chosen = await service.chooseBundleDestination('devnotes-2026-08-27.json');

    expect(chosen).toBe('C:/out/devnotes-2026-08-27.json');
    expect(adapter.saveCalls[0].defaultPath).toBe('devnotes-2026-08-27.json');
  });

  it('reports a failed save dialog as no destination', async () => {
    adapter.throwOnSave = new Error('no bridge');

    expect(await service.chooseBundleDestination('x.json')).toBeNull();
  });
  it('leaves an attachment destination unfiltered', async () => {
    adapter.savePath = 'C:/out/capture.png';

    const chosen = await service.chooseDestination('capture.png');

    expect(chosen).toBe('C:/out/capture.png');
    expect(adapter.saveCalls[0].filters).toBeUndefined();
    expect(adapter.saveCalls[0].defaultPath).toBe('capture.png');
  });
});
