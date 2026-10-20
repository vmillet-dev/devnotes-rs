import { ComponentFixture, TestBed } from '@angular/core/testing';
import { By } from '@angular/platform-browser';
import { describe, expect, it, vi } from 'vitest';
import { HttpOrigin, InheritedScriptsRow, ScriptParts } from '@core/model/http.model';
import { CodeEditorComponent } from '@notes/overlays/note-editor-overlay/code-editor/code-editor.component';
import { provideAppTesting } from '@testing/testing.providers';
import { ScriptEditorComponent } from './script-editor.component';

const API: HttpOrigin = { kind: 'collection', id: 'api', name: 'API Paiements' };
const INVOICES: HttpOrigin = { kind: 'folder', id: 'invoices', name: 'Factures' };

describe('ScriptEditorComponent', () => {
  let fixture: ComponentFixture<ScriptEditorComponent>;
  let changes: ScriptParts[];
  let opened: HttpOrigin[];

  async function render(
    scripts: ScriptParts,
    above: readonly InheritedScriptsRow[] | null = null,
    line: number | null = null,
  ) {
    TestBed.resetTestingModule();
    TestBed.configureTestingModule({ imports: [ScriptEditorComponent], providers: [provideAppTesting()] });
    fixture = TestBed.createComponent(ScriptEditorComponent);
    fixture.componentRef.setInput('scripts', scripts);
    fixture.componentRef.setInput('owner', 'tab-1');
    fixture.componentRef.setInput('above', above);
    fixture.componentRef.setInput('line', line);
    fixture.componentRef.setInput('requestName', 'Payer');
    changes = [];
    opened = [];
    fixture.componentInstance.scriptsChange.subscribe((next) => changes.push(next));
    fixture.componentInstance.levelOpened.subscribe((from) => opened.push(from));
    fixture.autoDetectChanges();
    await fixture.whenStable();
  }

  const root = () => fixture.nativeElement as HTMLElement;
  const levels = () =>
    [...root().querySelectorAll<HTMLElement>('[data-testid="http-script-level"]')].map((level) =>
      level.textContent?.replace(/\s+/g, ' ').trim(),
    );
  const editor = () =>
    fixture.debugElement.query(By.directive(CodeEditorComponent))?.componentInstance as
      CodeEditorComponent | undefined;

  it('marks the phase that has code, and says the order before the send and back after it', async () => {
    await render({ before: '', after: 'test()' }, [
      { from: API, scripts: { before: 'login()', after: '' } },
      { from: INVOICES, scripts: { before: '', after: '' } },
    ]);

    const marked = [...root().querySelectorAll('[data-testid="segment"]')].map(
      (segment) => segment.querySelector('[data-testid="segment-mark"]') !== null,
    );
    expect(marked).toEqual([false, true]);
    expect(levels()).toEqual(['collection API Paiements', 'dossier Factures · aucun', 'cette requête Payer']);

    root().querySelector<HTMLButtonElement>('[data-segment-id="after"]')!.click();
    await fixture.whenStable();
    expect(root().querySelector('[data-testid="http-scripts"]')?.getAttribute('data-phase')).toBe('after');
    expect(levels()).toEqual([
      'cette requête Payer',
      'dossier Factures · aucun',
      'collection API Paiements · aucun',
    ]);

    root().querySelector<HTMLButtonElement>('[data-level="collection"]')!.click();
    expect(opened).toEqual([API]);
  });

  it('says no order for a folder’s or a collection’s own', async () => {
    await render({ before: '', after: '' });
    expect(root().querySelector('[data-testid="http-script-order"]')).toBeNull();
  });

  it('hands every edit out under its phase, and stops marking the line once edited', async () => {
    await render({ before: 'throw 1;', after: 'kept()' }, [], 1);
    await vi.waitFor(() => expect(editor()).toBeDefined());
    expect(editor()!.focusedLines()).toEqual(new Set([1]));

    editor()!.changed.emit('const ok = 1;');
    await fixture.whenStable();
    expect(changes).toEqual([{ before: 'const ok = 1;', after: 'kept()' }]);
    expect(editor()!.focusedLines().size).toBe(0);
  });

  it('writes an example at the end of the script, a blank line after what is there', async () => {
    await render({ before: 'const a = 1;\n\n', after: '' });

    root().querySelector<HTMLButtonElement>('[data-testid="http-script-examples"]')!.click();
    await fixture.whenStable();
    root().querySelector<HTMLButtonElement>('[data-example="setVariable"]')!.click();
    expect(changes.at(-1)).toEqual({ before: "const a = 1;\n\nenv.set('name', 'value');\n", after: '' });

    fixture.componentRef.setInput('scripts', { before: '', after: '' });
    root().querySelector<HTMLButtonElement>('[data-testid="http-script-examples"]')!.click();
    await fixture.whenStable();
    root().querySelector<HTMLButtonElement>('[data-example="testStatus"]')!.click();
    expect(changes.at(-1)?.before).toBe(
      'test("Le statut est 200", () => {\n  expect(res.status).toBe(200);\n});\n',
    );
  });

  it('has a block for each of the five examples', async () => {
    await render({ before: '', after: '' });
    const written: string[] = [];
    for (const example of ['readResponse', 'addHeader', 'sendRequest']) {
      fixture.componentRef.setInput('scripts', { before: '', after: '' });
      root().querySelector<HTMLButtonElement>('[data-testid="http-script-examples"]')!.click();
      await fixture.whenStable();
      root().querySelector<HTMLButtonElement>(`[data-example="${example}"]`)!.click();
      written.push(changes.at(-1)!.before);
    }
    expect(written[0]).toContain('res.json()');
    expect(written[1]).toContain("req.headers.set('X-Request-Id'");
    expect(written[2]).toContain('await pm.sendRequest');
  });
});
