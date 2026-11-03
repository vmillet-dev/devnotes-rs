import { ComponentFixture, TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';
import { ScriptStop } from '@core/model/http.model';
import { provideAppTesting } from '@testing/testing.providers';
import { ScriptStopComponent } from './script-stop.component';

describe('ScriptStopComponent', () => {
  let fixture: ComponentFixture<ScriptStopComponent>;
  let opened: number;

  beforeEach(() => {
    TestBed.resetTestingModule();
    TestBed.configureTestingModule({
      imports: [ScriptStopComponent],
      providers: [provideAppTesting()],
    });
    opened = 0;
  });

  async function render(stop: ScriptStop, openable = true) {
    fixture = TestBed.createComponent(ScriptStopComponent);
    fixture.componentRef.setInput('stop', stop);
    fixture.componentRef.setInput('openable', openable);
    fixture.componentInstance.opened.subscribe(() => opened++);
    fixture.autoDetectChanges();
    await fixture.whenStable();
  }

  const text = (testId: string) =>
    (fixture.nativeElement as HTMLElement)
      .querySelector(`[data-testid="${testId}"]`)
      ?.textContent?.replace(/\s+/g, ' ')
      .trim() ?? null;
  const title = () => (fixture.nativeElement as HTMLElement).querySelector('.title')?.textContent?.trim();

  it('says whose script stopped the send, what it threw and on which line, and opens it', async () => {
    await render({
      kind: 'error',
      level: 'collection',
      script: 'before',
      name: 'API Paiements',
      owner: 'api',
      line: 3,
      message: 'ReferenceError: tokn is not defined',
    });

    expect(title()).toBe('Le script avant l’envoi de la collection « API Paiements » a échoué');
    expect(text('http-script-stop-message')).toBe('ReferenceError: tokn is not defined');
    expect(text('http-script-stop-where')).toBe('Ligne 3 · la requête n’a pas été envoyée');
    (fixture.nativeElement as HTMLElement)
      .querySelector<HTMLButtonElement>('[data-testid="http-script-open"]')!
      .click();
    expect(opened).toBe(1);
  });

  it('words a stop of its own for a time or memory run out, and a folder after the answer', async () => {
    await render({
      kind: 'timeout',
      level: 'folder',
      script: 'after',
      name: 'Factures',
      line: null,
      message: 'x',
    });

    expect(title()).toBe('Le script après la réponse du dossier « Factures » a échoué');
    expect(text('http-script-stop-message')).toBe('Arrêté : il a pris trop de temps.');
    expect(text('http-script-stop-where')).toBeNull();
  });

  it('names the host when no script was reached, and offers no way to it from the history', async () => {
    await render({ kind: 'host', level: null, script: 'before', line: null, message: 'spawn failed' }, false);

    expect(title()).toBe('Les scripts n’ont pas pu être lancés');
    expect(text('http-script-stop-where')).toBe('La requête n’a pas été envoyée');
    expect(text('http-script-open')).toBeNull();
  });

  it('gives the line alone after the answer, which came', async () => {
    await render({ kind: 'error', level: 'request', script: 'after', line: 2, message: 'TypeError: x' });

    expect(title()).toBe('Le script après la réponse a échoué');
    expect(text('http-script-stop-where')).toBe('Ligne 2');
  });

  /** « const = ; » read « a échoué — SyntaxError: variable name expected » (HSC-05). */
  it('says a script that does not read is not valid JavaScript, followed by the engine words', async () => {
    await render({
      kind: 'syntax',
      level: 'request',
      script: 'before',
      line: 1,
      message: 'variable name expected',
    });

    expect(title()).toBe('Le script avant l’envoi n’est pas du JavaScript valide');
    expect(text('http-script-stop-message')).toBe('variable name expected');
    expect(text('http-script-stop-where')).toBe('Ligne 1 · la requête n’a pas été envoyée');

    await render({
      kind: 'syntax',
      level: 'folder',
      script: 'tests',
      name: 'Factures',
      line: 4,
      message: 'x',
    });
    expect(title()).toBe('Les tests du dossier « Factures » ne sont pas du JavaScript valide');
  });
});
