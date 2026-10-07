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
