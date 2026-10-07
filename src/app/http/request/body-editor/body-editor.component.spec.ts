import { ComponentFixture, TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { BodyAnswer, RequestBodyDraft } from '@core/model/http.model';
import { FakeFileDialog } from '@testing/fake-file-dialog';
import { FakePrettier } from '@testing/fake-prettier';
import { provideAppTesting } from '@testing/testing.providers';
import { BodyEditorComponent } from './body-editor.component';

describe('BodyEditorComponent', () => {
  let fixture: ComponentFixture<BodyEditorComponent>;
  let emitted: RequestBodyDraft[];
  let fileDialog: FakeFileDialog;
  let prettier: FakePrettier;

  async function render(body: RequestBodyDraft, answer: BodyAnswer | null = null) {
    TestBed.resetTestingModule();
    fileDialog = new FakeFileDialog();
    prettier = new FakePrettier();
    TestBed.configureTestingModule({
      imports: [BodyEditorComponent],
      providers: [provideAppTesting({ fileDialog, prettier })],
    });
    fixture = TestBed.createComponent(BodyEditorComponent);
    fixture.componentRef.setInput('body', body);
    fixture.componentRef.setInput('answer', answer);
    emitted = [];
    fixture.componentInstance.bodyChange.subscribe((next) => emitted.push(next));
    fixture.autoDetectChanges();
    await fixture.whenStable();
  }

  const el = <E extends HTMLElement = HTMLElement>(selector: string): E =>
    fixture.nativeElement.querySelector(selector);
  const kind = (id: string) =>
    el<HTMLButtonElement>(`[data-testid="segmented-http-body"] [data-segment-id="${id}"]`).click();

  beforeEach(() => render({ kind: 'none' }));

  it('switches kind, keeping the text between JSON and plain text alone', async () => {
    kind('json');
    kind('form');
    kind('multipart');
    kind('binary');
    expect(emitted).toEqual([
      { kind: 'json', text: '' },
      { kind: 'form', fields: [] },
      { kind: 'multipart', parts: [] },
      { kind: 'binary', path: '' },
    ]);

    await render({ kind: 'json', text: '{"a":1}' });
    kind('text');
    kind('none');
    expect(emitted).toEqual([{ kind: 'text', text: '{"a":1}' }, { kind: 'none' }]);
  });

  it('says what the body is sent as, and why a JSON one does not read', async () => {
    await render(
      { kind: 'json', text: '{"a":' },
      {
        contentType: 'application/json',
        problem: { reason: 'unexpectedEnd', line: 1, column: 6, offset: 5 },
      },
    );

    expect(el('[data-testid="http-body-type"]').textContent?.trim()).toBe('Envoyé en application/json');
    expect(el('[data-testid="http-body-problem"]').textContent?.replace(/\s+/g, ' ').trim()).toBe(
      'JSON invalide ligne 1, colonne 6 : le document s’arrête trop tôt.',
    );
    expect(el<HTMLButtonElement>('[data-testid="http-body-format"]').disabled).toBe(true);
  });

  it('formats a JSON body through Prettier, and hands on what is typed', async () => {
    await render({ kind: 'json', text: '{"a":1}' }, { contentType: 'application/json', problem: null });
    expect(el('[data-testid="http-body-valid"]')).not.toBeNull();
    prettier.answer = { kind: 'formatted', text: '{ "a": 1 }\n', cursor: 0, changedLines: [0] };

    el<HTMLButtonElement>('[data-testid="http-body-format"]').click();
    await vi.waitFor(() => expect(emitted).toEqual([{ kind: 'json', text: '{ "a": 1 }\n' }]));

    const area = el<HTMLTextAreaElement>('[data-testid="http-body-text"]');
    area.value = '[]';
    area.dispatchEvent(new Event('input'));
    expect(emitted.at(-1)).toEqual({ kind: 'json', text: '[]' });
  });

  it('takes a binary body as a path chosen from the disk', async () => {
    await render({ kind: 'binary', path: '' });
    expect(el('[data-testid="http-body-path"]').textContent?.trim()).toBe('Aucun fichier choisi');
    fileDialog.openPath = 'C:/photos/facture.pdf';

    el<HTMLButtonElement>('[data-testid="http-body-choose"]').click();
    await vi.waitFor(() => expect(emitted).toEqual([{ kind: 'binary', path: 'C:/photos/facture.pdf' }]));
  });

  it('edits a form and a multipart body in their tables', async () => {
    await render({ kind: 'form', fields: [] });
    const key = el<HTMLInputElement>('[data-testid="http-form-table"] [data-testid="http-kv-key"]');
    key.value = 'q';
    key.dispatchEvent(new Event('input'));
    expect(emitted[0]).toEqual({
      kind: 'form',
      fields: [{ enabled: true, key: 'q', value: '', description: '' }],
    });

    await render({ kind: 'multipart', parts: [] });
    const part = el<HTMLInputElement>('[data-testid="http-part-key"]');
    part.value = 'file';
    part.dispatchEvent(new Event('input'));
    expect(emitted[0]).toMatchObject({ kind: 'multipart', parts: [{ key: 'file', file: false }] });
  });
});
