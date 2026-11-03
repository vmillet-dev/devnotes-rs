import { ComponentFixture, TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { BodyAnswer, BodyKind, HttpMethod, RequestBodyDraft } from '@core/model/http.model';
import { FakeFileDialog } from '@testing/fake-file-dialog';
import { FakePrettier } from '@testing/fake-prettier';
import { provideAppTesting } from '@testing/testing.providers';
import { BodyEditorComponent } from './body-editor.component';

describe('BodyEditorComponent', () => {
  let fixture: ComponentFixture<BodyEditorComponent>;
  let emitted: RequestBodyDraft[];
  let chosen: BodyKind[];
  let fileDialog: FakeFileDialog;
  let prettier: FakePrettier;

  const said = (answer: Partial<BodyAnswer>): BodyAnswer => ({
    contentType: null,
    problem: null,
    unusual: false,
    ...answer,
  });

  async function render(
    body: RequestBodyDraft,
    answer: BodyAnswer | null = null,
    method: HttpMethod = 'POST',
  ) {
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
    fixture.componentRef.setInput('method', method);
    emitted = [];
    chosen = [];
    fixture.componentInstance.bodyChange.subscribe((next) => emitted.push(next));
    fixture.componentInstance.kindChosen.subscribe((next) => chosen.push(next));
    fixture.autoDetectChanges();
    await fixture.whenStable();
  }

  const el = <E extends HTMLElement = HTMLElement>(selector: string): E =>
    fixture.nativeElement.querySelector(selector);
  const kind = (id: string) =>
    el<HTMLButtonElement>(`[data-testid="segmented-http-body"] [data-segment-id="${id}"]`).click();

  beforeEach(() => render({ kind: 'none' }));

  /** What the kind starts with is the tab's to say: it holds what each one had when left. */
  it('asks for another kind without building its body, and not for the one it shows', async () => {
    kind('none');
    kind('json');
    kind('binary');
    expect(chosen).toEqual(['json', 'binary']);
    expect(emitted).toEqual([]);
  });

  /** A `POST` switched to `GET` kept its body and sent it: nothing said so. */
  it('says a body is unusual on its method, and offers to send none without emptying it', async () => {
    await render({ kind: 'json', text: '{"a":1}' }, said({ contentType: 'application/json' }));
    expect(el('[data-testid="http-body-unusual"]')).toBeNull();

    await render({ kind: 'json', text: '{"a":1}' }, said({ unusual: true }), 'GET');
    expect(el('[data-testid="http-body-unusual"]').textContent).toContain(
      'Un GET avec un corps : beaucoup de serveurs l’ignorent ou le refusent.',
    );

    el<HTMLButtonElement>('[data-testid="http-body-to-none"]').click();
    expect(chosen).toEqual(['none']);
    // The text is not touched: it is set aside by whoever holds the tab.
    expect(emitted).toEqual([]);
    expect(el<HTMLTextAreaElement>('[data-testid="http-body-text"]').value).toBe('{"a":1}');

    await render({ kind: 'text', text: 'a' }, said({ unusual: true }), 'HEAD');
    expect(el('[data-testid="http-body-unusual"]').textContent).toContain('Un HEAD avec un corps');
  });

  it('says what the body is sent as, and why a JSON one does not read', async () => {
    await render(
      { kind: 'json', text: '{"a":' },
      said({
        contentType: 'application/json',
        problem: { reason: 'unexpectedEnd', line: 1, column: 6, offset: 5 },
      }),
    );

    expect(el('[data-testid="http-body-type"]').textContent?.trim()).toBe('Envoyé en application/json');
    expect(el('[data-testid="http-body-problem"]').textContent?.replace(/\s+/g, ' ').trim()).toBe(
      'JSON invalide ligne 1, colonne 6 : le document s’arrête trop tôt.',
    );
    expect(el<HTMLButtonElement>('[data-testid="http-body-format"]').disabled).toBe(true);
  });

  it('formats a JSON body through Prettier, and hands on what is typed', async () => {
    await render({ kind: 'json', text: '{"a":1}' }, said({ contentType: 'application/json' }));
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
