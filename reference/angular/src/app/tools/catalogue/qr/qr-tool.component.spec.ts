import { TestBed } from '@angular/core/testing';
import { describe, expect, it, vi } from 'vitest';
import { QrAnswer, QrRequest } from '@core/model/tool-answers.model';
import { FILE_DIALOG_ADAPTER } from '@core/services/dialogs/file-dialog.service';
import { StatusNotifier } from '@core/services/notifications/status.service';
import { FakeFileDialog } from '@testing/fake-file-dialog';
import { ToolHarness, renderTool } from '@testing/tool-harness';
import { QrToolComponent } from './qr-tool.component';

const CODE: QrAnswer = {
  kind: 'code',
  payload: 'https://exemple.fr/doc',
  version: 2,
  modules: 25,
  bytes: 22,
  capacity: 26,
  recoverable: 15,
  side: 33,
  path: 'M4,4h7v1h-7z',
  missingScheme: false,
};

describe('QrToolComponent', () => {
  type Harness = ToolHarness<QrToolComponent>;

  const render = (answer: QrAnswer = CODE) =>
    renderTool(QrToolComponent, (tools) => {
      tools.qr = answer;
    });

  const segment = (harness: Harness, kind: string, id: string) =>
    harness.element<HTMLButtonElement>(`[data-testid="segmented-${kind}"] [data-segment-id="${id}"]`);

  const lastRequest = (harness: Harness) => harness.tools.requestsOf('describe_qr_code').at(-1) as QrRequest;

  async function choose(harness: Harness, kind: string, id: string): Promise<void> {
    segment(harness, kind, id).click();
    await harness.settle();
  }

  it('opens on a URL at level M with the quiet zone of the spec', async () => {
    const harness = await render({ kind: 'empty' });

    expect(lastRequest(harness)).toEqual({
      content: { kind: 'url', url: '' },
      correction: 'medium',
      margin: 4,
    });
    expect(harness.element('[data-testid="qr-preview"]').classList).toContain('empty');
    expect(harness.element<HTMLButtonElement>('[data-testid="qr-svg"]').disabled).toBe(true);
    expect(harness.tool.result()).toBeNull();
  });

  it('builds each kind of content from its own form', async () => {
    const harness = await render();

    await harness.type('qr-url', 'https://exemple.fr', 'describe_qr_code');
    expect(lastRequest(harness).content).toEqual({ kind: 'url', url: 'https://exemple.fr' });

    await choose(harness, 'qr-kind', 'text');
    await harness.type('qr-text', 'Bonjour', 'describe_qr_code');
    expect(lastRequest(harness).content).toEqual({ kind: 'text', text: 'Bonjour' });

    await choose(harness, 'qr-kind', 'email');
    await harness.type('qr-to', 'ada@exemple.fr', 'describe_qr_code');
    await harness.type('qr-subject', 'Réunion', 'describe_qr_code');
    await harness.type('qr-body', 'À demain', 'describe_qr_code');
    expect(lastRequest(harness).content).toEqual({
      kind: 'email',
      to: 'ada@exemple.fr',
      subject: 'Réunion',
      body: 'À demain',
    });

    await choose(harness, 'qr-kind', 'contact');
    await harness.type('qr-name', 'Ada Lovelace', 'describe_qr_code');
    await harness.type('qr-organisation', 'DevNotes', 'describe_qr_code');
    expect(lastRequest(harness).content).toEqual({
      kind: 'contact',
      name: 'Ada Lovelace',
      phone: '',
      email: '',
      organisation: 'DevNotes',
    });
  });

  it('builds a Wi-Fi network with its security, and no password field to fill when it is open', async () => {
    const harness = await render();
    await choose(harness, 'qr-kind', 'wifi');

    await harness.type('qr-ssid', 'Maison', 'describe_qr_code');
    await harness.type('qr-password', 'secret', 'describe_qr_code');
    harness.element<HTMLInputElement>('[data-testid="qr-hidden"]').click();
    await choose(harness, 'qr-security', 'wep');
    await vi.waitFor(() =>
      expect(lastRequest(harness).content).toEqual({
        kind: 'wifi',
        ssid: 'Maison',
        password: 'secret',
        security: 'wep',
        hidden: true,
      }),
    );

    await choose(harness, 'qr-security', 'open');
    expect(harness.element<HTMLInputElement>('[data-testid="qr-password"]').disabled).toBe(true);
  });

  it('asks again for each correction level and margin, the margin held to 0–16', async () => {
    const harness = await render();

    await choose(harness, 'qr-correction', 'high');
    await harness.type('qr-margin', '40', 'describe_qr_code');

    expect(lastRequest(harness)).toMatchObject({ correction: 'high', margin: 16 });
    expect(harness.element<HTMLInputElement>('[data-testid="qr-margin"]').value).toBe('16');
  });

  it('draws the code Rust answers, and its facts', async () => {
    const harness = await render();
    await harness.type('qr-url', 'https://exemple.fr/doc', 'describe_qr_code');

    const svg = harness.element('[data-testid="qr-code"]');
    expect(svg.getAttribute('viewBox')).toBe('0 0 33 33');
    expect(svg.querySelector('path')?.getAttribute('d')).toBe('M4,4h7v1h-7z');
    expect(harness.element('[data-testid="qr-facts"]').textContent).toContain('2 · 25 × 25 modules');
    expect(harness.element('[data-testid="qr-facts"]').textContent).toContain('22 octets sur 26');
    expect(harness.element('[data-testid="qr-facts"]').textContent).toContain('jusqu’à 15 %');
  });

  it.each(['dark', 'light'])('keeps the plate white and the modules black in the %s theme', async (theme) => {
    document.documentElement.dataset['theme'] = theme;
    const harness = await render();
    await harness.type('qr-url', 'https://exemple.fr/doc', 'describe_qr_code');

    const plate = harness.element('[data-testid="qr-preview"]');
    expect(getComputedStyle(plate).backgroundColor).toBe('rgb(255, 255, 255)');
    expect(getComputedStyle(plate.querySelector('path')!).fill).toBe('rgb(0, 0, 0)');
    delete document.documentElement.dataset['theme'];
  });

  it('says a URL without a scheme and a payload past the maximum', async () => {
    let harness = await render({ ...CODE, missingScheme: true });
    await harness.type('qr-url', 'exemple.fr', 'describe_qr_code');
    expect(harness.element('[data-testid="qr-missing-scheme"]')).not.toBeNull();

    harness = await render({ kind: 'tooLong', bytes: 1274, maximum: 1273 });
    await harness.type('qr-url', 'https://exemple.fr', 'describe_qr_code');
    expect(harness.element('[data-testid="qr-too-long"]').textContent).toContain('1273');
    expect(harness.element('[data-testid="qr-code"]')).toBeNull();
  });

  it('downloads the code on screen in the format asked', async () => {
    const harness = await render();
    const dialog = TestBed.inject(FILE_DIALOG_ADAPTER) as FakeFileDialog;
    dialog.savePath = 'C:/out/qr-code.png';
    harness.tools.savedCode = { kind: 'saved', bytes: 812 };
    await harness.type('qr-url', 'https://exemple.fr/doc', 'describe_qr_code');

    harness.element<HTMLButtonElement>('[data-testid="qr-png"]').click();
    await vi.waitFor(() => expect(harness.tools.requestsOf('save_qr_code')).toHaveLength(1));

    expect(dialog.saveCalls[0]?.defaultPath).toBe('qr-code.png');
    expect(harness.tools.requestsOf('save_qr_code')[0]).toEqual({
      request: lastRequest(harness),
      format: 'png',
      path: 'C:/out/qr-code.png',
    });
    await vi.waitFor(() => expect(TestBed.inject(StatusNotifier).status()?.key).toBe('tools.qr.saved'));
  });

  it('copies the image natively', async () => {
    const harness = await render();
    await harness.type('qr-url', 'https://exemple.fr/doc', 'describe_qr_code');

    harness.element<HTMLButtonElement>('[data-testid="qr-copy"]').click();

    await vi.waitFor(() => expect(TestBed.inject(StatusNotifier).status()?.key).toBe('tools.qr.copied'));
    expect(harness.tools.requestsOf('copy_qr_code')).toEqual([lastRequest(harness)]);
  });

  it('keeps the payload as a note, warning when it carries a Wi-Fi password', async () => {
    const harness = await render();
    await harness.type('qr-url', 'https://exemple.fr/doc', 'describe_qr_code');
    expect(harness.tool.result()).toEqual({
      title: { key: 'tools.qr.noteTitle' },
      kind: 'snippet',
      language: 'txt',
      content: 'https://exemple.fr/doc',
    });

    await choose(harness, 'qr-kind', 'wifi');
    await harness.type('qr-ssid', 'Maison', 'describe_qr_code');
    await harness.type('qr-password', 'secret', 'describe_qr_code');
    await vi.waitFor(() => expect(harness.tool.result()?.warning).toEqual({ key: 'tools.qr.saveWarning' }));
  });

  it('fills a URL as its sample and empties every field on Vider, the password too', async () => {
    const harness = await render();
    harness.tool.sample();
    await harness.settle();
    await vi.waitFor(() =>
      expect(lastRequest(harness).content).toEqual({ kind: 'url', url: 'https://exemple.fr/doc' }),
    );

    await choose(harness, 'qr-kind', 'wifi');
    await harness.type('qr-password', 'secret', 'describe_qr_code');
    harness.tool.clear();
    await harness.settle();

    expect(harness.element<HTMLInputElement>('[data-testid="qr-password"]').value).toBe('');
    await vi.waitFor(() => expect(lastRequest(harness).content).toMatchObject({ ssid: '', password: '' }));
  });

  it('forgets the Wi-Fi password when the tool is left, and keeps the rest', async () => {
    const harness = await render();
    await choose(harness, 'qr-kind', 'wifi');
    await harness.type('qr-ssid', 'Maison', 'describe_qr_code');
    await harness.type('qr-password', 'secret', 'describe_qr_code');

    harness.fixture.destroy();
    const again = TestBed.createComponent(QrToolComponent);
    again.autoDetectChanges();
    await again.whenStable();

    const field = (testid: string) =>
      (again.nativeElement as HTMLElement).querySelector<HTMLInputElement>(`[data-testid="${testid}"]`)!
        .value;
    expect([field('qr-ssid'), field('qr-password')]).toEqual(['Maison', '']);
  });
});
