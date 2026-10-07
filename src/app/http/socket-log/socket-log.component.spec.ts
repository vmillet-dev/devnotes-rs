import { ComponentFixture, TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';
import { WEBSOCKET_SUBSCRIBER } from '@core/ipc/websocket-events';
import { EMPTY_PARTS, SocketEvent, WebsocketEvent } from '@core/model/http.model';
import { HttpSocketsStore } from '@core/services/http/http-sockets.store';
import { FakeHttpRepository } from '@testing/fake-http-repository';
import { provideAppTesting } from '@testing/testing.providers';
import { SocketLogComponent } from './socket-log.component';

describe('SocketLogComponent', () => {
  let fixture: ComponentFixture<SocketLogComponent>;
  let play: (event: WebsocketEvent) => void;

  const el = (testId: string) =>
    (fixture.nativeElement as HTMLElement).querySelector<HTMLElement>(`[data-testid="${testId}"]`);
  const all = (testId: string) => [
    ...(fixture.nativeElement as HTMLElement).querySelectorAll<HTMLElement>(`[data-testid="${testId}"]`),
  ];
  const event = async (kind: SocketEvent) => {
    const id = TestBed.inject(HttpSocketsStore).states().get('Flux')!.socketId;
    play({ id, at: '2026-10-07T12:00:00.000Z', event: kind });
    await fixture.whenStable();
  };

  beforeEach(async () => {
    TestBed.resetTestingModule();
    TestBed.configureTestingModule({
      imports: [SocketLogComponent],
      providers: [
        provideAppTesting({ httpRepository: new FakeHttpRepository() }),
        {
          provide: WEBSOCKET_SUBSCRIBER,
          useValue: (handler: (event: WebsocketEvent) => void) => {
            play = handler;
            return Promise.resolve(() => undefined);
          },
        },
      ],
    });
    fixture = TestBed.createComponent(SocketLogComponent);
    fixture.componentRef.setInput('tabKey', 'Flux');
    fixture.autoDetectChanges();
    await TestBed.inject(HttpSocketsStore).connect({
      key: 'Flux',
      place: null,
      draft: { parts: EMPTY_PARTS },
    });
    await fixture.whenStable();
  });

  it('says where the socket stands, and logs each message with its direction and size', async () => {
    expect(el('http-socket-phase')?.textContent?.trim()).toBe('Connexion…');
    await event({ kind: 'opened', url: 'wss://flux.exemple.fr', protocol: 'chat' });
    await event({ kind: 'sent', text: 'ping', size: 4 });
    await event({ kind: 'received', text: '', size: 12, binary: true, cut: false });

    expect(el('http-socket-phase')?.textContent?.trim()).toBe('Connecté');
    expect(all('http-socket-entry').map((entry) => entry.dataset['kind'])).toEqual([
      'opened',
      'sent',
      'received',
    ]);
    expect(all('http-socket-entry')[0]?.textContent).toContain('Connecté à wss://flux.exemple.fr');
    expect(all('http-socket-entry')[1]?.textContent).toContain('↑');
    expect(all('http-socket-entry')[2]?.textContent).toContain('Message binaire');
    expect(all('http-socket-entry')[2]?.textContent).toContain('12 o');
  });

  it('filters the log by its words, and empties it', async () => {
    await event({ kind: 'received', text: '{"tick":1}', size: 10, binary: false, cut: false });
    await event({ kind: 'received', text: '{"alert":true}', size: 14, binary: false, cut: false });

    const filter = el('http-socket-filter') as HTMLInputElement;
    filter.value = 'ALERT';
    filter.dispatchEvent(new Event('input'));
    await fixture.whenStable();
    expect(all('http-socket-entry')).toHaveLength(1);

    el('http-socket-clear')!.click();
    await fixture.whenStable();
    expect(el('http-socket-empty')).not.toBeNull();
  });
});
