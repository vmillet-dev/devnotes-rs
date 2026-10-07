import { ComponentFixture, TestBed } from '@angular/core/testing';
import { beforeEach, describe, expect, it, vi } from 'vitest';
import { CookieDomain, JarCookie } from '@core/model/http.model';
import { FakeHttpRepository } from '@testing/fake-http-repository';
import { provideAppTesting } from '@testing/testing.providers';
import { CookieJarDialogComponent } from './cookie-jar-dialog.component';

function cookie(name: string, domain: string, change: Partial<JarCookie> = {}): JarCookie {
  return {
    name,
    value: 'v',
    domain,
    hostOnly: false,
    path: '/',
    expires: null,
    secure: false,
    httpOnly: false,
    sameSite: null,
    ...change,
  };
}

const JAR: CookieDomain[] = [
  {
    domain: 'api.exemple.fr',
    cookies: [
      {
        id: 'session',
        cookie: cookie('session', 'api.exemple.fr', {
          secure: true,
          httpOnly: true,
          sameSite: 'Lax',
          hostOnly: true,
        }),
      },
      { id: 'theme', cookie: cookie('theme', 'api.exemple.fr', { expires: '2026-12-01T10:00:00.000Z' }) },
    ],
  },
  { domain: 'other.fr', cookies: [{ id: 'x', cookie: cookie('x', 'other.fr') }] },
];

describe('CookieJarDialogComponent', () => {
  let fixture: ComponentFixture<CookieJarDialogComponent>;
  let http: FakeHttpRepository;
  let closed: number;

  beforeEach(async () => {
    TestBed.resetTestingModule();
    http = new FakeHttpRepository();
    http.jar = JAR;
    TestBed.configureTestingModule({
      imports: [CookieJarDialogComponent],
      providers: [provideAppTesting({ httpRepository: http })],
    });
    fixture = TestBed.createComponent(CookieJarDialogComponent);
    closed = 0;
    fixture.componentInstance.closed.subscribe(() => closed++);
    fixture.autoDetectChanges();
    await vi.waitFor(() => expect(all('[data-testid="http-cookie"]')).toHaveLength(3));
  });

  const el = <E extends HTMLElement = HTMLElement>(selector: string): E =>
    document.body.querySelector(selector) as E;
  const all = (selector: string): HTMLElement[] => [...document.body.querySelectorAll<HTMLElement>(selector)];

  it('lists the jar by domain, each cookie with its attributes and expiry', () => {
    expect(all('[data-testid="http-cookies-domain"]').map((domain) => domain.dataset['domain'])).toEqual([
      'api.exemple.fr',
      'other.fr',
    ]);
    const [session, theme] = all('[data-testid="http-cookie"]');
    expect(session!.textContent).toContain('Secure');
    expect(session!.textContent).toContain('SameSite=Lax');
    expect(session!.textContent).toContain('Hôte seul');
    expect(session!.textContent).toContain('Session');
    expect(theme!.textContent).toContain('2026');
  });

  it('deletes a cookie, then a domain, and reads the jar again each time', async () => {
    all('[data-testid="http-cookie-remove"]')[0]!.click();
    await vi.waitFor(() => expect(all('[data-testid="http-cookie"]')).toHaveLength(2));
    expect(http.callsOf('deleteCookie')).toEqual([['session']]);

    all('[data-testid="http-cookies-domain-remove"]')[1]!.click();
    await vi.waitFor(() => expect(all('[data-testid="http-cookie"]')).toHaveLength(1));
    expect(http.callsOf('deleteCookieDomain')).toEqual([['other.fr']]);
  });

  it('says how many go before emptying the jar, and can be cancelled', async () => {
    el<HTMLButtonElement>('[data-testid="http-cookies-clear"]').click();
    await vi.waitFor(() =>
      expect(el('[data-testid="http-cookies-clear-count"]')?.textContent).toContain('3'),
    );
    el<HTMLButtonElement>('[data-testid="http-cookies-clear-cancel"]').click();
    await fixture.whenStable();
    expect(http.callsOf('clearCookies')).toEqual([]);

    el<HTMLButtonElement>('[data-testid="http-cookies-clear"]').click();
    await vi.waitFor(() => expect(el('[data-testid="http-cookies-clear-confirm"]')).not.toBeNull());
    el<HTMLButtonElement>('[data-testid="http-cookies-clear-confirm"]').click();
    await vi.waitFor(() => expect(el('[data-testid="http-cookies-empty"]')).not.toBeNull());
    expect(http.callsOf('clearCookies')).toHaveLength(1);
    expect(el<HTMLButtonElement>('[data-testid="http-cookies-clear"]').disabled).toBe(true);

    el<HTMLButtonElement>('[data-testid="http-cookies-close"]').click();
    expect(closed).toBe(1);
  });
});
