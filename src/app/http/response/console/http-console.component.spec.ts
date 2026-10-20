import { ComponentFixture, TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';
import { HTTP_CONSOLE_SUBSCRIBER } from '@core/ipc/http-console-events';
import { ConsoleEvent, ConsoleLine } from '@core/model/http-console.model';
import { EMPTY_PARTS } from '@core/model/http.model';
import { HttpConsoleStore } from '@core/services/http/http-console.store';
import { HttpSendStore } from '@core/services/http/http-send.store';
import { FakeHttpRepository } from '@testing/fake-http-repository';
import { provideAppTesting } from '@testing/testing.providers';
import { HttpConsoleComponent } from './http-console.component';

describe('HttpConsoleComponent', () => {
  let fixture: ComponentFixture<HttpConsoleComponent>;
  let play: (event: ConsoleEvent) => void;
  let opened: number;
  let sendId: string;

  beforeEach(async () => {
    TestBed.resetTestingModule();
    const http = new FakeHttpRepository();
    http.pendingSend = new Promise(() => undefined);
    TestBed.configureTestingModule({
      imports: [HttpConsoleComponent],
      providers: [
        provideAppTesting({ httpRepository: http }),
        {
          provide: HTTP_CONSOLE_SUBSCRIBER,
          useValue: (handler: (event: ConsoleEvent) => void) => {
            play = handler;
            return Promise.resolve(() => undefined);
          },
        },
      ],
    });
    void TestBed.inject(HttpSendStore).send({
      key: 'Payer',
      place: null,
      requestId: null,
      draft: { name: 'Payer', kind: 'http', method: 'POST', parts: EMPTY_PARTS },
    });
    sendId = TestBed.inject(HttpSendStore).states().get('Payer')!.sendId;
    fixture = TestBed.createComponent(HttpConsoleComponent);
    fixture.componentRef.setInput('tabKey', 'Payer');
    fixture.componentRef.setInput('requestName', 'Payer');
    fixture.componentRef.setInput('currentSendId', sendId);
    opened = 0;
    fixture.componentInstance.exchangeOpened.subscribe(() => opened++);
    fixture.autoDetectChanges();
    await fixture.whenStable();
  });

  const say = async (line: ConsoleLine, id = sendId) => {
    play({ id, at: '2026-10-20T12:02:11.998Z', line });
    await fixture.whenStable();
  };
  const root = () => fixture.nativeElement as HTMLElement;
  const text = (testId: string) =>
    root().querySelector(`[data-testid="${testId}"]`)?.textContent?.replace(/\s+/g, ' ').trim() ?? null;
  const lines = () =>
    [...root().querySelectorAll('[data-testid="http-console-line"]')].map((line) =>
      line.textContent?.replace(/\s+/g, ' ').trim(),
    );
  const by = { kind: 'before' as const, level: 'collection' as const, name: 'API Paiements' };

  it('folds to a line that counts what it holds, and unfolds on it', async () => {
    await say({ kind: 'out', method: 'POST', url: 'https://api/pay', headers: [], by: null });
    await say({ kind: 'log', level: 'log', text: 'signé', by });
    await say({ kind: 'test', name: 'devise', expected: '"eur"', actual: '"usd"', message: null, by: null });
    // Another tab's send is not this one's.
    await say({ kind: 'log', level: 'log', text: 'ailleurs', by: null }, 'Autre#9');

    expect(text('http-console-summary')).toBe('1 échange · 1 log');
    expect(text('http-console-errors')).toBe('1 erreur');
    root().querySelector<HTMLButtonElement>('[data-testid="http-console-unfold"]')!.click();
    await fixture.whenStable();
    expect(TestBed.inject(HttpConsoleStore).open()).toBe(true);
    expect(lines()).toHaveLength(3);
  });

  it('says each kind of line with its time and whose it is, a secret apart', async () => {
    TestBed.inject(HttpConsoleStore).toggle();
    await say({
      kind: 'out',
      method: 'POST',
      url: 'https://api/pay',
      headers: [{ key: 'Authorization', value: 'Bearer ••••••••' }],
      by: null,
    });
    await say({ kind: 'in', status: 402, reason: 'Payment Required', millis: 203, size: 318, by: null });
    await say({ kind: 'out', method: 'GET', url: 'https://api/rates', headers: [], by });
    await say({ kind: 'failed', code: 'httpRefused', params: {}, by });
    await say({ kind: 'log', level: 'warn', text: 'jeton : {{accessToken}}', by });
    await say({
      kind: 'test',
      name: 'trace',
      expected: null,
      actual: null,
      message: 'pas d’en-tête',
      by: null,
    });
    await say({
      kind: 'stopped',
      stop: { kind: 'timeout', level: 'request', script: 'before', message: 'x' },
    });
    await say({
      kind: 'stopped',
      stop: { kind: 'error', level: 'folder', script: 'after', name: 'Factures', message: 'boom' },
    });
    await say({ kind: 'missed', path: 'body.token', variable: 'token' });

    const said = lines();
    expect(said[0]).toContain('POST https://api/pay');
    expect(said[0]).toContain('Payer');
    expect(text('http-console-sub')).toBe('Authorization: Bearer ••••••••');
    expect(said[1]).toContain('402 Payment Required · 203 ms · 318 o');
    expect(said[2]).toContain('pm.sendRequest · API Paiements');
    expect(said[3]).toContain('La connexion a été refusée');
    expect(root().querySelector('.secret')?.textContent).toBe('{{accessToken}}');
    expect(said[4]).toContain('avant l’envoi · API Paiements');
    expect(said[5]).toContain('trace : pas d’en-tête');
    expect(said[6]).toContain('Le script avant l’envoi a échoué : Arrêté : il a pris trop de temps.');
    expect(said[7]).toContain('Le script après la réponse du dossier « Factures » a échoué : boom');
    expect(said[8]).toContain('body.token ne se lit plus dans la réponse');
    expect(said[0]?.startsWith(new Date('2026-10-20T12:02:11.998Z').toTimeString().slice(0, 8))).toBe(true);
  });

  it('filters, opens its own exchange, and clears', async () => {
    TestBed.inject(HttpConsoleStore).toggle();
    await say({ kind: 'out', method: 'GET', url: 'https://api', headers: [], by: null });
    await say({ kind: 'log', level: 'log', text: 'hello', by });
    await say({ kind: 'missed', path: 'body.x', variable: 'x' });

    root().querySelector<HTMLButtonElement>('[data-filter="logs"]')!.click();
    await fixture.whenStable();
    expect(lines()).toHaveLength(1);
    root().querySelector<HTMLButtonElement>('[data-filter="errors"]')!.click();
    await fixture.whenStable();
    expect(lines()[0]).toContain('body.x');
    root().querySelector<HTMLButtonElement>('[data-filter="all"]')!.click();
    await fixture.whenStable();

    root().querySelector<HTMLButtonElement>('[data-testid="http-console-exchange"]')!.click();
    expect(opened).toBe(1);

    root().querySelector<HTMLButtonElement>('[data-testid="http-console-clear"]')!.click();
    await fixture.whenStable();
    expect(lines()).toEqual([]);
    expect(text('http-console-lines')).toContain('Rien encore');
    root().querySelector<HTMLButtonElement>('[data-testid="http-console-fold"]')!.click();
    await fixture.whenStable();
    expect(text('http-console-unfold')).toContain('Console');
  });

  it('follows a tab saved for the first time, and forgets a tab closed', async () => {
    await say({ kind: 'log', level: 'log', text: 'hello', by: null });
    const store = TestBed.inject(HttpConsoleStore);
    store.rekey('Payer', 'pay-id');
    expect(store.linesOf('Payer')).toEqual([]);
    expect(store.linesOf('pay-id')).toHaveLength(1);
    store.rekey('nothing', 'else');
    store.forget('pay-id');
    expect(store.lines().size).toBe(0);
    store.forget('pay-id');
  });
});
