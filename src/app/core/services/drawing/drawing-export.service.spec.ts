import { TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';
import { ErrorNotifier } from '@core/services/errors/error-notifier.service';
import { StatusNotifier } from '@core/services/notifications/status.service';
import { FakeFileDialog } from '@testing/fake-file-dialog';
import { FakeToolsRepository } from '@testing/fake-tools-repository';
import { provideAppTesting } from '@testing/testing.providers';
import { DrawingExportService, base64Of, fileNameOf } from './drawing-export.service';

describe('DrawingExportService', () => {
  let dialog: FakeFileDialog;
  let tools: FakeToolsRepository;
  let service: DrawingExportService;

  beforeEach(() => {
    dialog = new FakeFileDialog();
    tools = new FakeToolsRepository();
    TestBed.configureTestingModule({
      providers: [provideAppTesting({ fileDialog: dialog, toolsRepository: tools })],
    });
    service = TestBed.inject(DrawingExportService);
  });

  it('writes a PNG where the user says, under the drawing’s name and its extension', async () => {
    dialog.savePath = 'C:\\exports\\Archi.png';
    tools.savedBytes = { kind: 'saved', bytes: 120 };

    await service.save('Archi', 'png', 'data:image/png;base64,AAAA');

    expect(dialog.saveCalls[0]).toEqual({
      defaultPath: 'Archi.png',
      filters: [{ name: 'PNG', extensions: ['png'] }],
    });
    expect(tools.requestsOf('save_bytes')).toEqual([
      { request: { text: 'data:image/png;base64,AAAA', reading: null }, path: 'C:\\exports\\Archi.png' },
    ]);
    expect(TestBed.inject(StatusNotifier).status()).toEqual({
      key: 'drawing.exported',
      params: { name: 'Archi.png' },
    });
  });

  it('writes an SVG as its UTF-8 text', async () => {
    dialog.savePath = '/home/me/plan.svg';

    await service.save('Plan', 'svg', '<svg>é</svg>');

    const [sent] = tools.requestsOf('save_bytes') as { request: { text: string } }[];
    expect(sent!.request.text).toBe(`data:image/svg+xml;base64,${base64Of('<svg>é</svg>')}`);
  });

  it('writes nothing when the dialog is cancelled, and says why a write failed', async () => {
    await service.save('Archi', 'png', 'data:image/png;base64,AAAA');
    expect(tools.requestsOf('save_bytes')).toEqual([]);

    dialog.savePath = '/locked/archi.png';
    tools.savedBytes = { kind: 'failed', problem: 'unwritable' };
    await service.save('Archi', 'png', 'data:image/png;base64,AAAA');

    expect(TestBed.inject(ErrorNotifier).notice()?.ref).toEqual({ key: 'tools.files.unwritable' });
  });
});

describe('fileNameOf', () => {
  it('keeps what a file name can hold, and names a drawing without a title', () => {
    expect(fileNameOf('API / DB: v2?')).toBe('API - DB- v2-');
    expect(fileNameOf('  ')).toBe('dessin');
  });
});

describe('base64Of', () => {
  it('encodes the UTF-8 bytes, a long text in slices', () => {
    expect(atob(base64Of('é'))).toBe('\u00c3\u00a9');
    expect(atob(base64Of('a'.repeat(70_000))).length).toBe(70_000);
  });
});
