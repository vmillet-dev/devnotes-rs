import { ComponentFixture, TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';
import { WebsocketParts } from '@core/model/http.model';
import { provideAppTesting } from '@testing/testing.providers';
import { WebsocketComposerComponent } from './websocket-composer.component';

describe('WebsocketComposerComponent', () => {
  let fixture: ComponentFixture<WebsocketComposerComponent>;
  let sent: string[];
  let changed: WebsocketParts[];

  beforeEach(async () => {
    TestBed.resetTestingModule();
    TestBed.configureTestingModule({
      imports: [WebsocketComposerComponent],
      providers: [provideAppTesting()],
    });
    fixture = TestBed.createComponent(WebsocketComposerComponent);
    fixture.componentRef.setInput('websocket', {
      protocols: [],
      messages: [{ name: 'ping', text: '{"type":"ping"}' }],
    });
    fixture.componentRef.setInput('open', false);
    sent = [];
    changed = [];
    fixture.componentInstance.sent.subscribe((text) => sent.push(text));
    fixture.componentInstance.websocketChange.subscribe((parts) => changed.push(parts));
    fixture.autoDetectChanges();
    await fixture.whenStable();
  });

  const el = <E extends HTMLElement = HTMLElement>(testId: string): E =>
    (fixture.nativeElement as HTMLElement).querySelector<E>(`[data-testid="${testId}"]`)!;
  const type = async (text: string) => {
    const field = el<HTMLTextAreaElement>('http-socket-message');
    field.value = text;
    field.dispatchEvent(new Event('input'));
    await fixture.whenStable();
  };

  it('sends only on an open socket', async () => {
    await type('hello');
    expect(el<HTMLButtonElement>('http-socket-send').disabled).toBe(true);

    fixture.componentRef.setInput('open', true);
    await fixture.whenStable();
    el('http-socket-send').click();
    el('http-socket-saved-send').click();
    expect(sent).toEqual(['hello', '{"type":"ping"}']);
  });

  /**
   * `Ctrl+Entrée` in the message did nothing of what was meant: the page answered the key by
   * connecting, whatever held the focus.
   */
  it('sends the message on Ctrl+Entrée when the socket is open, and says to connect when it is not', async () => {
    const reached: KeyboardEvent[] = [];
    const listen = (event: KeyboardEvent) => reached.push(event);
    document.addEventListener('keydown', listen);
    const press = (init: KeyboardEventInit) => {
      const event = new KeyboardEvent('keydown', { key: 'Enter', bubbles: true, cancelable: true, ...init });
      el('http-socket-message').dispatchEvent(event);
      return event;
    };
    try {
      await type('{"type":"ping"}');

      // Not open: said, and the page is not left to connect behind the user's back.
      expect(press({ ctrlKey: true }).defaultPrevented).toBe(true);
      await fixture.whenStable();
      expect(sent).toEqual([]);
      expect(reached).toEqual([]);
      expect(el('http-socket-not-open').textContent?.trim()).toBe('Connectez le WebSocket pour envoyer.');

      fixture.componentRef.setInput('open', true);
      await fixture.whenStable();
      expect(el('http-socket-not-open')).toBeNull();
      press({ metaKey: true });
      expect(sent).toEqual(['{"type":"ping"}']);
      expect(reached).toEqual([]);

      // Enter alone is a new line — a message is often several lines of JSON — and is the field's.
      const plain = press({});
      expect(plain.defaultPrevented).toBe(false);
      expect(press({ ctrlKey: true, shiftKey: true }).defaultPrevented).toBe(false);
      expect(sent).toHaveLength(1);
      expect(reached).toHaveLength(2);
    } finally {
      document.removeEventListener('keydown', listen);
    }
  });

  it('carries the key on its button, as a request does', () => {
    expect(el('http-socket-send').querySelector('kbd')?.textContent).toBe('Ctrl ↵');
  });

  it('keeps a message under its first line, and lets one go', async () => {
    await type('{"type":"subscribe",\n "channel":"ticks"}');
    el('http-socket-keep').click();
    expect(changed[0]?.messages.at(-1)).toEqual({
      name: '{"type":"subscribe",',
      text: '{"type":"subscribe",\n "channel":"ticks"}',
    });

    el('http-socket-saved-remove').click();
    expect(changed[1]?.messages).toEqual([]);
  });
});
