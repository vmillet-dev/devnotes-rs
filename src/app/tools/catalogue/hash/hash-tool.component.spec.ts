import { TestBed } from '@angular/core/testing';
import { describe, expect, it, vi } from 'vitest';
import { HashAnswer } from '@core/model/tool-answers.model';
import { FakeToolsRepository } from '@testing/fake-tools-repository';
import { ToolHarness, renderTool } from '@testing/tool-harness';
import { HashToolComponent } from './hash-tool.component';

const HASHED: HashAnswer = {
  kind: 'hashed',
  bytes: 62,
  endsWithNewline: false,
  digests: [
    { algorithm: 'sha1', bits: 160, value: '1fbd6946' },
    { algorithm: 'sha256', bits: 256, value: 'bfc0d2dc' },
  ],
  verdict: null,
};

describe('HashToolComponent', () => {
  const answer = (tools: FakeToolsRepository): void => {
    tools.hashAnswer = HASHED;
  };

  const text = (harness: ToolHarness<HashToolComponent>, testid: string): string =>
    harness.element(`[data-testid="${testid}"]`)?.textContent?.replace(/\s+/g, ' ').trim() ?? '';

  const rows = (harness: ToolHarness<HashToolComponent>) =>
    harness.all('[data-testid="output-row"]').map((row) => row.dataset['name']);

  async function typeKey(harness: ToolHarness<HashToolComponent>, key: string): Promise<void> {
    const hmac = harness.element<HTMLInputElement>('[data-testid="hash-hmac"]');
    if (!hmac.checked) {
      hmac.click();
      await harness.settle();
    }
    await harness.type('hash-key', key, 'hash_input');
  }

  it('digests a text in every algorithm, in hex, and measures it', async () => {
    const harness = await renderTool(HashToolComponent, answer);

    await harness.type('hash-input', 'payload', 'hash_input');

    expect(harness.tools.requestsOf('hash_input')).toEqual([
      {
        input: { kind: 'text', text: 'payload' },
        algorithms: ['md5', 'sha1', 'sha256', 'sha384', 'sha512', 'sha3-256'],
        encoding: 'hex',
        key: null,
        expected: null,
      },
    ]);
    expect(rows(harness)).toEqual(['SHA-1', 'SHA-256']);
    expect(text(harness, 'hash-measure')).toBe('62 octets · UTF-8 · aucun saut de ligne final (echo -n)');
  });

  it('marks MD5 and SHA-1 obsolete, and gives each digest its length', async () => {
    const harness = await renderTool(HashToolComponent, answer);

    await harness.type('hash-input', 'payload', 'hash_input');

    const row = (name: string) => harness.element(`[data-name="${name}"]`);
    expect(row('SHA-1').querySelector('[data-testid="hash-obsolete"]')?.textContent?.trim()).toBe('obsolète');
    expect(row('SHA-256').querySelector('[data-testid="hash-obsolete"]')).toBeNull();
    expect(row('SHA-256').querySelector('.meta')?.textContent).toBe('256 bits');
  });

  it('keys the digests once a key is typed, and names them HMAC', async () => {
    const harness = await renderTool(HashToolComponent, answer);
    await harness.type('hash-input', 'payload', 'hash_input');

    await typeKey(harness, 'whsec_9f2c');

    expect(harness.tools.requestsOf('hash_input').at(-1)).toMatchObject({ key: 'whsec_9f2c' });
    expect(rows(harness)).toEqual(['HMAC-SHA1', 'HMAC-SHA256']);
    expect(harness.element<HTMLInputElement>('[data-testid="hash-key"]').type).toBe('password');
  });

  it('names the digests on screen after the key they were computed with, not the one being typed', async () => {
    const harness = await renderTool(HashToolComponent, answer);
    await harness.type('hash-input', 'payload', 'hash_input');
    harness.element<HTMLInputElement>('[data-testid="hash-hmac"]').click();
    await harness.settle();

    const key = harness.element<HTMLInputElement>('[data-testid="hash-key"]');
    key.value = 'whsec_9f2c';
    key.dispatchEvent(new Event('input'));
    await harness.settle();

    expect(rows(harness)).toEqual(['SHA-1', 'SHA-256']);
    expect(harness.tool.result()!.content).toBe('SHA-1    1fbd6946\nSHA-256  bfc0d2dc');
    await vi.waitFor(() =>
      expect(harness.tools.requestsOf('hash_input').at(-1)).toMatchObject({ key: 'whsec_9f2c' }),
    );
    await harness.settle();
    expect(rows(harness)).toEqual(['HMAC-SHA1', 'HMAC-SHA256']);
  });

  it('keeps the digests as a note, never the key', async () => {
    const harness = await renderTool(HashToolComponent, answer);
    await harness.type('hash-input', 'payload', 'hash_input');
    await typeKey(harness, 'whsec_9f2c');

    const result = harness.tool.result()!;
    expect(result.content).toBe('HMAC-SHA1    1fbd6946\nHMAC-SHA256  bfc0d2dc');
    expect(JSON.stringify(result)).not.toContain('whsec_9f2c');
  });

  /** Left and found again, the tool has its text and its switch back, and never its key. */
  it('forgets the key when it is left', async () => {
    const first = await renderTool(HashToolComponent, answer);
    await first.type('hash-input', 'payload', 'hash_input');
    await typeKey(first, 'whsec_9f2c');
    first.fixture.destroy();

    const again = TestBed.createComponent(HashToolComponent);
    again.autoDetectChanges();
    await again.whenStable();

    expect(again.nativeElement.querySelector('[data-testid="hash-input"]').value).toBe('payload');
    expect(again.nativeElement.querySelector('[data-testid="hash-hmac"]').checked).toBe(true);
    expect(again.nativeElement.querySelector('[data-testid="hash-key"]').value).toBe('');
  });

  it('says which digest a pasted signature is, and marks its row', async () => {
    const harness = await renderTool(HashToolComponent, (tools) => {
      tools.hashAnswer = { ...HASHED, verdict: { kind: 'matches', algorithm: 'sha256', encoding: 'hex' } };
    });
    await harness.type('hash-input', 'payload', 'hash_input');
    await typeKey(harness, 'whsec_9f2c');

    await harness.type('hash-expected', 'BFC0D2DC', 'hash_input');

    expect(text(harness, 'hash-verdict')).toBe('Correspond à HMAC-SHA256, en hex');
    expect(harness.element('[data-name="HMAC-SHA256"]').closest('app-result-row')!.classList).toContain(
      'matched',
    );
  });

  it('says a signature matches nothing, what it looks like, and what to check', async () => {
    const harness = await renderTool(HashToolComponent, (tools) => {
      tools.hashAnswer = {
        ...HASHED,
        verdict: {
          kind: 'noMatch',
          shape: { kind: 'hex', characters: 64, algorithms: ['sha256', 'sha3-256'] },
        },
      };
    });
    await harness.type('hash-input', 'payload', 'hash_input');

    await harness.type('hash-expected', 'e94cbc7c', 'hash_input');

    expect(text(harness, 'hash-verdict')).toBe('Aucune correspondance');
    expect(text(harness, 'hash-shape')).toBe('64 caractères hexadécimaux : SHA-256 ou SHA3-256');
    expect(harness.fixture.nativeElement.textContent).toContain('si HMAC doit être activé');
  });

  /** Nothing to hash yet: a pasted digest is still read, by its shape alone, and no key goes. */
  it('reads a digest pasted over an empty input by its shape', async () => {
    const harness = await renderTool(HashToolComponent, (tools) => {
      tools.hashAnswer = { kind: 'shaped', shape: { kind: 'base64', bytes: 7, algorithms: [] } };
    });
    harness.element<HTMLInputElement>('[data-testid="hash-hmac"]').click();
    await harness.settle();
    harness.element<HTMLInputElement>('[data-testid="hash-key"]').value = 'whsec_9f2c';
    harness.element('[data-testid="hash-key"]').dispatchEvent(new Event('input'));

    await harness.type('hash-expected', 'qZk+NkcGgW', 'hash_input');

    expect(harness.tools.requestsOf('hash_input').at(-1)).toMatchObject({
      input: null,
      key: null,
      expected: 'qZk+NkcGgW',
    });
    expect(text(harness, 'hash-shape')).toBe(
      'Base64 de 7 octets : aucune empreinte de l’outil n’a cette longueur.',
    );
    expect(harness.element('[data-testid="hash-verdict"]')).toBeNull();
    expect(harness.fixture.nativeElement.textContent).toContain('Une HMAC a la forme du hash');
  });

  it('says a text is no digest at all', async () => {
    const harness = await renderTool(HashToolComponent, (tools) => {
      tools.hashAnswer = { kind: 'shaped', shape: { kind: 'unknown' } };
    });

    await harness.type('hash-expected', 'pas une empreinte', 'hash_input');

    expect(text(harness, 'hash-shape')).toBe('Ni hexadécimal ni base64 : ce n’est pas une empreinte.');
  });

  it('adds and removes algorithms in their own order, and warns about the weak ones', async () => {
    const harness = await renderTool(HashToolComponent, answer);
    await harness.type('hash-input', 'payload', 'hash_input');
    const pill = (id: string) => harness.element<HTMLButtonElement>(`[data-algorithm="${id}"]`);

    pill('sha1').click();
    pill('md5').click();
    await vi.waitFor(() =>
      expect(harness.tools.requestsOf('hash_input').at(-1)).toMatchObject({
        algorithms: ['sha256', 'sha384', 'sha512', 'sha3-256'],
      }),
    );
    await harness.settle();
    expect(harness.fixture.nativeElement.textContent).not.toContain('MD5 et SHA-1 ne protègent plus rien');

    pill('md5').click();
    await vi.waitFor(() =>
      expect(harness.tools.requestsOf('hash_input').at(-1)).toMatchObject({
        algorithms: ['md5', 'sha256', 'sha384', 'sha512', 'sha3-256'],
      }),
    );
    await harness.settle();

    expect(pill('md5').getAttribute('aria-pressed')).toBe('true');
    expect(pill('sha1').getAttribute('aria-pressed')).toBe('false');
    expect(harness.fixture.nativeElement.textContent).toContain('MD5 et SHA-1 ne protègent plus rien');
  });

  it('says a file cannot be read', async () => {
    const harness = await renderTool(HashToolComponent, (tools) => {
      tools.hashAnswer = { kind: 'failed', problem: 'notFound' };
    });

    await harness.type('hash-input', 'x', 'hash_input');

    expect(text(harness, 'hash-problem')).toBe('Ce fichier est introuvable.');
  });
});
