import { ComponentFixture, TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it } from 'vitest';
import { DEFAULT_TRANSPORT, Transport, TransportSettings } from '@core/model/http.model';
import { provideAppTesting } from '@testing/testing.providers';
import { TransportEditorComponent } from './transport-editor.component';

describe('TransportEditorComponent', () => {
  let fixture: ComponentFixture<TransportEditorComponent>;
  let changes: TransportSettings[];

  async function render(
    settings: TransportSettings,
    above: Transport = DEFAULT_TRANSPORT,
    aboveIsDefault = false,
  ) {
    TestBed.resetTestingModule();
    TestBed.configureTestingModule({ imports: [TransportEditorComponent], providers: [provideAppTesting()] });
    fixture = TestBed.createComponent(TransportEditorComponent);
    fixture.componentRef.setInput('settings', settings);
    fixture.componentRef.setInput('above', above);
    fixture.componentRef.setInput('aboveIsDefault', aboveIsDefault);
    changes = [];
    fixture.componentInstance.settingsChange.subscribe((change) => {
      changes.push(change);
      fixture.componentRef.setInput('settings', change);
    });
    fixture.autoDetectChanges();
    await fixture.whenStable();
  }

  const el = <E extends HTMLElement = HTMLElement>(selector: string): E =>
    fixture.nativeElement.querySelector(selector) as E;
  const all = (selector: string): HTMLElement[] => [...fixture.nativeElement.querySelectorAll(selector)];

  beforeEach(() => render({}));

  it('shows what each unset field inherits beside it', () => {
    const from = all('[data-testid="http-transport-from"]').map((cell) => cell.textContent?.trim());
    expect(from[0]).toContain('30000 ms');
    expect(from.every((text) => text?.startsWith('Hérité'))).toBe(true);
    expect(el<HTMLInputElement>('[data-testid="http-transport-timeoutMs"]').placeholder).toBe('30000');
    expect(el('[data-testid="http-transport-insecure"]')).toBeNull();
  });

  it('says « Par défaut » where nothing is above', async () => {
    await render({}, DEFAULT_TRANSPORT, true);
    expect(el('[data-testid="http-transport-from"]').textContent).toContain('Par défaut');
  });

  it('sets a number, raised to its floor, and clears it back to inheriting', () => {
    const timeout = el<HTMLInputElement>('[data-testid="http-transport-timeoutMs"]');
    timeout.value = '5';
    timeout.dispatchEvent(new Event('change'));
    timeout.value = '';
    timeout.dispatchEvent(new Event('change'));
    const redirects = el<HTMLInputElement>('[data-testid="http-transport-maxRedirects"]');
    redirects.value = 'abc';
    redirects.dispatchEvent(new Event('change'));

    expect(changes).toEqual([
      { timeoutMs: 100 },
      { timeoutMs: null },
      { timeoutMs: null, maxRedirects: null },
    ]);
  });

  it('turns a switch on, off, or back to what is above', async () => {
    const pick = async (id: string) => {
      el<HTMLButtonElement>('[data-testid="http-transport-verifyTls"]').click();
      await fixture.whenStable();
      document.body
        .querySelector<HTMLButtonElement>(`[data-testid="choice-option"][data-option-id="${id}"]`)!
        .click();
      await fixture.whenStable();
    };
    await pick('off');
    await pick('on');
    await pick('inherit');

    expect(changes).toEqual([{ verifyTls: false }, { verifyTls: true }, { verifyTls: null }]);
  });

  it('shows its own values, and warns while certificates go unchecked', async () => {
    await render({ timeoutMs: 2000, verifyTls: false, useCookies: true });

    expect(el<HTMLInputElement>('[data-testid="http-transport-timeoutMs"]').value).toBe('2000');
    expect(el('[data-testid="http-transport-verifyTls"]').textContent).toContain('Non');
    expect(el('[data-testid="http-transport-useCookies"]').textContent).toContain('Oui');
    expect(el('[data-testid="http-transport-insecure"]')).not.toBeNull();

    await render({}, { ...DEFAULT_TRANSPORT, verifyTls: false });
    expect(el('[data-testid="http-transport-insecure"]')).not.toBeNull();
  });
});
