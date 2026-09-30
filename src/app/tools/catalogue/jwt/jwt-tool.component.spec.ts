import { TestBed } from '@angular/core/testing';
import { describe, expect, it, vi } from 'vitest';
import { JwtAnswer } from '@core/model/tool-answers.model';
import { FakeToolsRepository } from '@testing/fake-tools-repository';
import { ToolHarness, renderTool } from '@testing/tool-harness';
import { JwtToolComponent } from './jwt-tool.component';

const TOKEN = 'eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiI0MiJ9.c2ln';
const day = 24 * 3_600_000;

const DECODED: JwtAnswer = {
  kind: 'decoded',
  header: '{\n  "alg": "HS256"\n}',
  payload: '{\n  "sub": "42"\n}',
  document: '{\n  "header": {},\n  "payload": {}\n}',
  algorithm: 'HS256',
  family: 'hmac',
  signature: 'c2ln',
  dates: [
    { name: 'exp', iso: '2026-09-27T12:00:00Z', epochMilliseconds: Date.now() - 3 * day - 60_000 },
    { name: 'iat', iso: '2026-09-20T12:00:00Z', epochMilliseconds: null },
  ],
  state: 'expired',
  verification: 'valid',
};

describe('JwtToolComponent', () => {
  const answering =
    (answer: JwtAnswer) =>
    (tools: FakeToolsRepository): void => {
      tools.jwt = answer;
    };

  const asked = (harness: ToolHarness<JwtToolComponent>) => harness.tools.requestsOf('decode_jwt');
  const text = (harness: ToolHarness<JwtToolComponent>, testid: string) =>
    harness.element(`[data-testid="${testid}"]`)?.textContent?.replace(/\s+/g, ' ').trim();

  it('asks nothing of an empty field', async () => {
    const harness = await renderTool(JwtToolComponent);

    expect(asked(harness)).toEqual([]);
    expect(harness.tool.result()).toBeNull();
  });

  it('decodes a token, says its state and when, and verifies it with the secret', async () => {
    const harness = await renderTool(JwtToolComponent, answering(DECODED));

    await harness.type('jwt-token', TOKEN, 'decode_jwt');
    await harness.type('jwt-secret', 'your-256-bit-secret', 'decode_jwt');

    expect(asked(harness).at(-1)).toEqual({
      token: TOKEN,
      secret: 'your-256-bit-secret',
      secretIsBase64: false,
    });
    expect(text(harness, 'jwt-state')).toBe('Expiré · il y a 3 jours');
    expect(harness.element('[data-testid="jwt-verification"]').classList).toContain('good');
    expect(text(harness, 'jwt-algorithm')).toBe('HS256 · HMAC');
    expect(harness.element('[data-testid="jwt-header"]').textContent).toBe('{\n  "alg": "HS256"\n}');
    expect(harness.all('[data-testid="jwt-date"]').map((date) => date.dataset['claim'])).toEqual([
      'exp',
      'iat',
    ]);
    expect(harness.element('[data-claim="iat"] .relative')).toBeNull();
  });

  it('masks the secret until asked, and reads it as base64 when told', async () => {
    const harness = await renderTool(JwtToolComponent, answering(DECODED));
    await harness.type('jwt-token', TOKEN, 'decode_jwt');
    const secret = harness.element<HTMLInputElement>('[data-testid="jwt-secret"]');

    expect(secret.type).toBe('password');
    harness.element<HTMLButtonElement>('[data-testid="jwt-secret-reveal"]').click();
    await harness.settle();
    expect(secret.type).toBe('text');

    const base64 = harness.element<HTMLInputElement>('[data-testid="jwt-secret-base64"]');
    base64.checked = true;
    base64.dispatchEvent(new Event('change'));
    await vi.waitFor(() => expect(asked(harness).at(-1)).toMatchObject({ secretIsBase64: true }));
  });

  it('names what is not verified here, and flags a token nobody signed', async () => {
    const harness = await renderTool(
      JwtToolComponent,
      answering({
        ...DECODED,
        family: 'rsa',
        algorithm: 'RS256',
        verification: 'notVerifiedHere',
        state: 'undated',
      }),
    );
    await harness.type('jwt-token', TOKEN, 'decode_jwt');
    expect(text(harness, 'jwt-verification')).toBe('RSA : signature non vérifiée ici.');
    expect(text(harness, 'jwt-state')).toBe('Sans date d’expiration');

    harness.tools.jwt = {
      ...DECODED,
      family: 'unsigned',
      algorithm: null,
      verification: 'unsigned',
      state: 'valid',
    };
    await harness.type('jwt-token', `${TOKEN}.`, 'decode_jwt');
    expect(harness.element('[data-testid="jwt-verification"]').classList).toContain('bad');
    expect(text(harness, 'jwt-algorithm')).toBe('— · aucune signature');
  });

  it('says a not-yet-valid token becomes valid when', async () => {
    const harness = await renderTool(
      JwtToolComponent,
      answering({
        ...DECODED,
        dates: [
          { name: 'nbf', iso: '2026-10-02T12:00:00Z', epochMilliseconds: Date.now() + 2 * day + 60_000 },
        ],
        state: 'notYetValid',
      }),
    );

    await harness.type('jwt-token', TOKEN, 'decode_jwt');

    expect(text(harness, 'jwt-state')).toBe('Pas encore valide · dans 2 jours');
  });

  it('says what is wrong with a token, and where', async () => {
    const harness = await renderTool(
      JwtToolComponent,
      answering({ kind: 'malformed', problem: 'segmentCount', segment: null, segments: 2, at: null }),
    );
    const problem = () => text(harness, 'jwt-problem');

    await harness.type('jwt-token', 'a.b', 'decode_jwt');
    expect(problem()).toContain('celui-ci en a 2');

    for (const [answer, expected] of [
      [{ kind: 'malformed', problem: 'segmentCount', segment: null, segments: 5, at: null }, 'JWE'],
      [
        { kind: 'malformed', problem: 'notBase64', segment: 'header', segments: 3, at: 4 },
        'En-tête : pas du base64url (caractère 4).',
      ],
      [
        { kind: 'malformed', problem: 'notBase64', segment: 'signature', segments: 3, at: null },
        'Signature : pas du base64url.',
      ],
      [
        { kind: 'malformed', problem: 'notJson', segment: 'payload', segments: 3, at: null },
        'Charge utile : pas du JSON.',
      ],
      [{ kind: 'malformed', problem: 'notObject', segment: 'header', segments: 3, at: null }, 'pas un objet'],
    ] as const) {
      harness.tools.jwt = answer;
      await harness.type('jwt-token', `${expected}.token`, 'decode_jwt');
      expect(problem()).toContain(expected);
    }
  });

  it('keeps the decoded header and payload as a note, after a warning, never the token', async () => {
    const harness = await renderTool(JwtToolComponent, answering(DECODED));

    await harness.type('jwt-token', TOKEN, 'decode_jwt');

    const result = harness.tool.result();
    expect(result).toEqual({
      title: { key: 'tools.jwt.noteTitle', params: { algorithm: 'HS256' } },
      kind: 'snippet',
      language: 'json',
      content: DECODED.kind === 'decoded' ? DECODED.document : '',
      warning: { key: 'tools.jwt.warning' },
    });
    expect(result?.content).not.toContain(TOKEN);
  });

  it('never keeps the token or the secret for the session', async () => {
    const harness = await renderTool(JwtToolComponent, answering(DECODED));
    await harness.type('jwt-token', TOKEN, 'decode_jwt');
    await harness.type('jwt-secret', 'secret', 'decode_jwt');

    const again = TestBed.createComponent(JwtToolComponent);
    again.autoDetectChanges();
    await again.whenStable();

    expect(again.nativeElement.querySelector('[data-testid="jwt-token"]').value).toBe('');
    expect(again.nativeElement.querySelector('[data-testid="jwt-secret"]').value).toBe('');
  });

  it('empties both fields on Vider and masks the secret again', async () => {
    const harness = await renderTool(JwtToolComponent, answering(DECODED));
    await harness.type('jwt-token', TOKEN, 'decode_jwt');
    harness.element<HTMLButtonElement>('[data-testid="jwt-secret-reveal"]').click();

    harness.tool.clear();
    await harness.settle();

    expect(harness.element<HTMLTextAreaElement>('[data-testid="jwt-token"]').value).toBe('');
    expect(harness.element<HTMLInputElement>('[data-testid="jwt-secret"]').type).toBe('password');
  });
});
