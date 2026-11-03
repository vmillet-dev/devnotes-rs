import { ComponentFixture, TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';
import { Attachment } from '@core/model/note.model';
import { provideTranslocoTesting } from '@testing/provide-transloco-testing';
import { AttachmentStripComponent } from './attachment-strip.component';

function attachment(overrides: Partial<Attachment> = {}): Attachment {
  return {
    id: 'attachment-1',
    noteId: 'note-1',
    fileName: 'capture.png',
    mimeType: 'image/png',
    byteSize: 2048,
    createdAt: new Date('2026-08-27T09:00:00Z'),
    ...overrides,
  };
}

describe('AttachmentStripComponent', () => {
  let fixture: ComponentFixture<AttachmentStripComponent>;

  function items(): HTMLElement[] {
    return [...fixture.nativeElement.querySelectorAll('.strip-item')];
  }

  /** The button of a row whose accessible label starts with `label`. */
  function actionOf(index: number, label: string): HTMLButtonElement {
    const found = [...items()[index].querySelectorAll<HTMLButtonElement>('button')].find((button) =>
      button.getAttribute('aria-label')?.startsWith(label),
    );
    if (!found) throw new Error(`No "${label}" action on row ${index}`);
    return found;
  }

  async function setAttachments(...list: Attachment[]): Promise<void> {
    fixture.componentRef.setInput('attachments', list);
    await fixture.whenStable();
  }

  beforeEach(async () => {
    TestBed.resetTestingModule();
    TestBed.configureTestingModule({
      imports: [AttachmentStripComponent],
      providers: [provideTranslocoTesting()],
    });
    fixture = TestBed.createComponent(AttachmentStripComponent);
    fixture.componentRef.setInput('attachments', [attachment()]);
    fixture.autoDetectChanges();
    await fixture.whenStable();
  });

  it('shows the file name and its size in kilobytes', () => {
    expect(items()[0].textContent).toContain('capture.png');
    expect(items()[0].textContent).toContain('2 Ko');
  });

  it('never reports a non-empty file as weighing nothing', async () => {
    await setAttachments(attachment({ byteSize: 12 }));

    expect(items()[0].textContent).toContain('1 Ko');
  });

  it('lets any attachment be opened with the system application', async () => {
    await setAttachments(
      attachment(),
      attachment({ id: 'attachment-2', fileName: 'dump.zip', mimeType: 'application/zip' }),
    );
    const opened: string[] = [];
    fixture.componentInstance.openRequested.subscribe((id) => opened.push(id));

    actionOf(0, 'Ouvrir').click();
    actionOf(1, 'Ouvrir').click();
    await fixture.whenStable();

    expect(opened).toEqual(['attachment-1', 'attachment-2']);
  });

  it('lets any attachment be saved somewhere else', async () => {
    let saved: string | undefined;
    fixture.componentInstance.saveRequested.subscribe((id) => (saved = id));

    actionOf(0, 'Enregistrer').click();
    await fixture.whenStable();

    expect(saved).toBe('attachment-1');
  });

  it('offers an inline preview only for an image', async () => {
    await setAttachments(
      attachment(),
      attachment({ id: 'attachment-2', fileName: 'dump.zip', mimeType: 'application/zip' }),
    );

    expect(items()[0].querySelector('[aria-pressed]')).not.toBeNull();
    expect(items()[1].querySelector('[aria-pressed]')).toBeNull();
  });

  it('asks for the bytes only when the preview is toggled', async () => {
    let toggled: string | undefined;
    fixture.componentInstance.previewToggled.subscribe((id) => (toggled = id));

    (items()[0].querySelector('[aria-pressed]') as HTMLButtonElement).click();
    await fixture.whenStable();

    expect(toggled).toBe('attachment-1');
  });

  it('renders the preview as a data URI once the bytes arrive', async () => {
    fixture.componentRef.setInput('previewId', 'attachment-1');
    await fixture.whenStable();
    expect(fixture.nativeElement.querySelector('.strip-preview-image')).toBeNull();

    fixture.componentRef.setInput('previewData', 'data:image/png;base64,AAA');
    await fixture.whenStable();

    const image: HTMLImageElement = fixture.nativeElement.querySelector('.strip-preview-image');
    expect(image.getAttribute('src')).toBe('data:image/png;base64,AAA');
    expect(image.getAttribute('alt')).toContain('capture.png');
  });

  it('asks for the enlarged view when the preview is clicked', async () => {
    fixture.componentRef.setInput('previewId', 'attachment-1');
    fixture.componentRef.setInput('previewData', 'data:image/png;base64,AAA');
    await fixture.whenStable();
    let zoomed = 0;
    fixture.componentInstance.zoomRequested.subscribe(() => (zoomed += 1));

    (fixture.nativeElement.querySelector('.strip-preview-zoom') as HTMLButtonElement).click();
    await fixture.whenStable();

    expect(zoomed).toBe(1);
  });

  it('asks for a confirmation before removing', async () => {
    let emitted = 0;
    fixture.componentInstance.removeRequested.subscribe(() => (emitted += 1));

    actionOf(0, 'Retirer').click();
    await fixture.whenStable();
    expect(emitted).toBe(0);

    actionOf(0, 'Retirer').click();
    await fixture.whenStable();
    expect(emitted).toBe(1);
  });

  it('explains the two other ways to attach when the strip is empty', async () => {
    await setAttachments();

    expect(fixture.nativeElement.querySelector('.strip-empty').textContent).toContain('Déposez');
  });

  it('marks the add button unavailable while a copy is running', async () => {
    fixture.componentRef.setInput('isBusy', true);
    await fixture.whenStable();

    const add: HTMLButtonElement = fixture.nativeElement.querySelector('.strip-add');
    expect(add.getAttribute('aria-disabled')).toBe('true');
    expect(add.textContent).toContain('Ajout');
  });
});
