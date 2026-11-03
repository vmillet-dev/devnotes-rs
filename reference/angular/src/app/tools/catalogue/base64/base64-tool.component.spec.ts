import { TestBed } from '@angular/core/testing';
import { describe, expect, it, vi } from 'vitest';
import { DecodeAnswer, Encodings } from '@core/model/tool-answers.model';
import { FILE_DIALOG_ADAPTER } from '@core/services/dialogs/file-dialog.service';
import { StatusNotifier } from '@core/services/notifications/status.service';
import { FakeFileDialog } from '@testing/fake-file-dialog';
import { ToolHarness, renderTool } from '@testing/tool-harness';
import { Base64ToolComponent } from './base64-tool.component';

const CAFE: Encodings = {
  characters: 4,
  bytes: 5,
  wide: { character: 'é', bytes: 2 },
  base64: 'Q2Fmw6k=',
  base64Url: 'Q2Fmw6k',
  base32: 'INQWNQ5J',
  hex: '43 61 66 c3 a9',
  binary: '01000011 01100001 01100110 11000011 10101001',
  decimal: '67 97 102 195 169',
};

/** A PNG of 32 by 20, as Rust answers it: named, sized, and handed over as a `data:` URL. */
const PICTURE: DecodeAnswer = {
  readAs: 'base64',
  guessed: true,
  also: [],
  decoded: { kind: 'binary', bytes: 1234, preview: '89 50 4e 47' },
  dataUri: null,
  image: {
    format: 'png',
    width: 32,
    height: 20,
    preview: { kind: 'shown', dataUrl: 'data:image/png;base64,iVBORw0KGgo=' },
  },
};

describe('Base64ToolComponent', () => {
  const segment = (harness: ToolHarness<Base64ToolComponent>, kind: string, id: string) =>
    harness.element<HTMLButtonElement>(`[data-testid="segmented-${kind}"] [data-segment-id="${id}"]`);

  const text = (harness: ToolHarness<Base64ToolComponent>, testid: string) =>
    harness.element(`[data-testid="${testid}"]`)?.textContent?.replace(/\s+/g, ' ').trim();

  /** The caption of a picture, part by part: its type, its size when stated, its weight. */
  const facts = (harness: ToolHarness<Base64ToolComponent>) =>
    harness.all('[data-testid="base64-image-facts"] span').map((part) => part.textContent?.trim());

  const rows = (harness: ToolHarness<Base64ToolComponent>) =>
    harness
      .all('[data-testid="output-row"]')
      .map((row) => [row.dataset['name'], row.querySelector('[data-testid="output-value"]')?.textContent]);

  it('writes a text in every encoding at once, and says how many bytes it is', async () => {
    const harness = await renderTool(Base64ToolComponent, (tools) => {
      tools.encodings = CAFE;
    });

    await harness.type('base64-input', 'Café', 'encode_bytes');

    expect(harness.tools.requestsOf('encode_bytes')).toEqual([
      { text: 'Café', spaced: true, uppercase: false },
    ]);
    expect(rows(harness)).toEqual([
      ['Base64', 'Q2Fmw6k='],
      ['Base64 URL', 'Q2Fmw6k'],
      ['Base32', 'INQWNQ5J'],
      ['Hexadécimal', '43 61 66 c3 a9'],
      ['Binaire', '01000011 01100001 01100110 11000011 10101001'],
      ['Octets décimaux', '67 97 102 195 169'],
    ]);
    expect(text(harness, 'base64-facts')).toBe('4 caractères · 5 octets en UTF-8 (« é » en occupe 2)');
    expect(harness.tool.result()?.content.split('\n')[0]).toBe('base64     Q2Fmw6k=');
  });

  it('asks again with the bytes apart or together, and the hexadecimal in capitals', async () => {
    const harness = await renderTool(Base64ToolComponent, (tools) => {
      tools.encodings = CAFE;
    });
    await harness.type('base64-input', 'Café', 'encode_bytes');

    harness.element<HTMLInputElement>('[data-testid="base64-spaced"]').click();
    harness.element<HTMLInputElement>('[data-testid="base64-uppercase"]').click();

    await vi.waitFor(() =>
      expect(harness.tools.requestsOf('encode_bytes').at(-1)).toEqual({
        text: 'Café',
        spaced: false,
        uppercase: true,
      }),
    );
  });

  it('encodes a file where it lies, in the two Base64 alone', async () => {
    const harness = await renderTool(Base64ToolComponent, (tools) => {
      tools.base64File = { kind: 'encoded', name: 'logo.png', bytes: 3, base64: 'AP8Q', base64Url: 'AP8Q' };
    });
    (TestBed.inject(FILE_DIALOG_ADAPTER) as FakeFileDialog).openPath = 'C:/images/logo.png';

    segment(harness, 'base64-source', 'file').click();
    await harness.settle();
    harness.element<HTMLButtonElement>('[data-testid="base64-pick-file"]').click();
    await vi.waitFor(() => expect(harness.tools.requestsOf('encode_base64_file')).toHaveLength(1));
    await harness.settle();

    expect(harness.tools.requestsOf('encode_base64_file')[0]).toBe('C:/images/logo.png');
    expect(text(harness, 'base64-file')).toBe('logo.png · 3 octets');
    expect(rows(harness)).toEqual([
      ['Base64', 'AP8Q'],
      ['Base64 URL', 'AP8Q'],
    ]);
    expect(harness.element('[data-testid="base64-spaced"]')).toBeNull();
  });

  it('decodes what it recognises, says what else it could be, and takes a forced reading', async () => {
    const harness = await renderTool(Base64ToolComponent, (tools) => {
      tools.decodedBytes = {
        readAs: 'hex',
        guessed: true,
        also: ['base64'],
        decoded: { kind: 'text', text: 'Café', bytes: 5 },
        dataUri: null,
        image: null,
      };
    });
    segment(harness, 'base64-direction', 'decode').click();

    await harness.type('base64-input', '43 61 66 c3 a9', 'decode_bytes');

    expect(harness.tools.requestsOf('decode_bytes')).toEqual([{ text: '43 61 66 c3 a9', reading: null }]);
    expect(text(harness, 'base64-reading')).toBe('Lu comme Hexadécimal · d’après sa forme');
    expect(text(harness, 'base64-also')).toBe('Se lit aussi comme Base64.');
    expect(text(harness, 'base64-output')).toBe('Café');
    expect(harness.tool.result()).toMatchObject({ content: 'Café' });

    segment(harness, 'base64-reading', 'base64').click();
    await vi.waitFor(() =>
      expect(harness.tools.requestsOf('decode_bytes').at(-1)).toEqual({
        text: '43 61 66 c3 a9',
        reading: 'base64',
      }),
    );
  });

  it('names the character a decoding stops at', async () => {
    const harness = await renderTool(Base64ToolComponent, (tools) => {
      tools.decodedBytes = {
        readAs: 'base64',
        guessed: true,
        also: [],
        decoded: { kind: 'invalid', problem: 'character', at: 6, character: '#' },
        dataUri: null,
        image: null,
      };
    });
    segment(harness, 'base64-direction', 'decode').click();

    await harness.type('base64-input', 'QUJD\nR#==', 'decode_bytes');

    expect(text(harness, 'base64-problem')).toContain('« # »');
    expect(text(harness, 'base64-problem')).toContain('caractère 7');
    expect(harness.tool.result()).toBeNull();
  });

  /**
   * Held to 860px, a long result ran down a strip in the middle of an empty window: the tool
   * takes the width it is given, what goes in on one side and what comes out on the other.
   */
  it('lays what goes in and what comes out in two columns, each under its title', async () => {
    const harness = await renderTool(Base64ToolComponent, (tools) => {
      tools.encodings = CAFE;
    });
    await harness.type('base64-input', 'Café', 'encode_bytes');

    const columns = harness.all('.columns > .column');
    expect(columns).toHaveLength(2);
    expect(columns[0]!.querySelector('[data-testid="base64-input"]')).not.toBeNull();
    expect(columns[0]!.querySelector('[data-testid="base64-encodings"]')).toBeNull();
    expect(columns[1]!.querySelector('[data-testid="base64-encodings"]')).not.toBeNull();
    expect(columns.map((column) => column.querySelector('h2')?.textContent?.trim())).toEqual([
      'Entrée',
      'Résultats',
    ]);
  });

  /** Decoded bytes that are a picture could be saved, then opened elsewhere to see what they were. */
  it('shows the picture a decoding is, with its type, its size and its weight', async () => {
    const harness = await renderTool(Base64ToolComponent, (tools) => {
      tools.decodedBytes = PICTURE;
    });
    segment(harness, 'base64-direction', 'decode').click();
    await harness.type('base64-input', 'iVBORw0KGgo=', 'decode_bytes');

    const picture = harness.element<HTMLImageElement>('[data-testid="base64-image-picture"]');
    // ⚠️ An `<img>`: drawn, never inlined — nothing an SVG holds would run.
    expect(picture.localName).toBe('img');
    expect(picture.getAttribute('src')).toBe('data:image/png;base64,iVBORw0KGgo=');
    expect(picture.getAttribute('alt')).toBe('Aperçu de l’image décodée (PNG)');
    // A few pixels a side: drawn larger, or it is a dot.
    expect(picture.classList.contains('small')).toBe(true);
    expect(harness.element('[data-testid="base64-image"]').dataset['format']).toBe('png');
    expect(facts(harness)).toEqual(['PNG', '32 × 20 px', '1234 octets']);
    // Under the result, in the column of what comes out.
    expect(harness.all('.columns > .column')[1]!.contains(picture)).toBe(true);
  });

  it('proposes the extension of the picture it recognised when saving it', async () => {
    const harness = await renderTool(Base64ToolComponent, (tools) => {
      tools.decodedBytes = PICTURE;
      tools.savedBytes = { kind: 'saved', bytes: 1234 };
    });
    const dialog = TestBed.inject(FILE_DIALOG_ADAPTER) as FakeFileDialog;
    dialog.savePath = 'C:/out/decoded.png';
    segment(harness, 'base64-direction', 'decode').click();
    await harness.type('base64-input', 'iVBORw0KGgo=', 'decode_bytes');

    harness.element<HTMLButtonElement>('[data-testid="base64-save-file"]').click();
    await vi.waitFor(() => expect(harness.tools.requestsOf('save_bytes')).toHaveLength(1));

    expect(dialog.saveCalls.at(-1)?.defaultPath).toBe('decoded.png');
  });

  it('shows an SVG under its own text, and a picture of a size its header does not state', async () => {
    const harness = await renderTool(Base64ToolComponent, (tools) => {
      tools.decodedBytes = {
        ...PICTURE,
        decoded: { kind: 'text', text: '<svg/>', bytes: 6 },
        image: {
          format: 'svg',
          width: null,
          height: null,
          preview: { kind: 'shown', dataUrl: 'data:image/svg+xml;base64,PHN2Zy8+' },
        },
      };
    });
    segment(harness, 'base64-direction', 'decode').click();
    await harness.type('base64-input', 'PHN2Zy8+', 'decode_bytes');

    expect(text(harness, 'base64-output')).toBe('<svg/>');
    expect(harness.element('[data-testid="base64-image-picture"]').getAttribute('src')).toBe(
      'data:image/svg+xml;base64,PHN2Zy8+',
    );
    expect(harness.element('[data-testid="base64-image-size"]')).toBeNull();
    // Of no stated size, it is not taken for a small one.
    expect(harness.element('[data-testid="base64-image-picture"]').classList.contains('small')).toBe(false);
    expect(facts(harness)).toEqual(['SVG', '6 octets']);
  });

  it('says a picture is too large to be shown rather than showing nothing', async () => {
    const harness = await renderTool(Base64ToolComponent, (tools) => {
      tools.decodedBytes = {
        ...PICTURE,
        image: { ...PICTURE.image!, preview: { kind: 'tooLarge', limit: 2 * 1024 * 1024 } },
      };
    });
    segment(harness, 'base64-direction', 'decode').click();
    await harness.type('base64-input', 'iVBORw0KGgo=', 'decode_bytes');

    expect(harness.element('[data-testid="base64-image-picture"]')).toBeNull();
    expect(text(harness, 'base64-image-too-large')).toContain('au-delà de 2 Mo');
    expect(facts(harness)).toContain('32 × 20 px');
  });

  it('shows no picture for bytes that are none', async () => {
    const harness = await renderTool(Base64ToolComponent, (tools) => {
      tools.decodedBytes = { ...PICTURE, image: null };
    });
    segment(harness, 'base64-direction', 'decode').click();
    await harness.type('base64-input', 'AP8Q', 'decode_bytes');

    expect(harness.element('[data-testid="base64-image"]')).toBeNull();
    // Unrecognised, they are saved as what they are: bytes.
    const dialog = TestBed.inject(FILE_DIALOG_ADAPTER) as FakeFileDialog;
    harness.element<HTMLButtonElement>('[data-testid="base64-save-file"]').click();
    await vi.waitFor(() => expect(dialog.saveCalls).toHaveLength(1));
    expect(dialog.saveCalls[0]?.defaultPath).toBe('decoded.bin');
  });

  /** Pasted whole, « data:image/png;base64,… » was refused as invalid Base64. */
  it('says a data: URI was read by its header, with the type it declares', async () => {
    const harness = await renderTool(Base64ToolComponent, (tools) => {
      tools.decodedBytes = { ...PICTURE, guessed: false, dataUri: 'image/png' };
    });
    segment(harness, 'base64-direction', 'decode').click();
    await harness.type('base64-input', 'data:image/png;base64,iVBORw0KGgo=', 'decode_bytes');

    expect(text(harness, 'base64-reading')).toBe('Lu comme Base64 · URI data: de type image/png');

    harness.tools.decodedBytes = { ...PICTURE, guessed: false, dataUri: '' };
    await harness.type('base64-input', 'data:;base64,iVBORw0KGgo=', 'decode_bytes');
    expect(text(harness, 'base64-reading')).toBe('Lu comme Base64 · URI data:');
  });

  it('offers to write bytes that are no text to a file, never to a note', async () => {
    const harness = await renderTool(Base64ToolComponent, (tools) => {
      tools.decodedBytes = {
        readAs: 'base64',
        guessed: true,
        also: [],
        decoded: { kind: 'binary', bytes: 3, preview: '00 ff 10' },
        dataUri: null,
        image: null,
      };
      tools.savedBytes = { kind: 'saved', bytes: 3 };
    });
    (TestBed.inject(FILE_DIALOG_ADAPTER) as FakeFileDialog).savePath = 'C:/out/decoded.bin';
    segment(harness, 'base64-direction', 'decode').click();
    await harness.type('base64-input', 'AP8Q', 'decode_bytes');

    expect(harness.tool.result()).toBeNull();
    harness.element<HTMLButtonElement>('[data-testid="base64-save-file"]').click();
    await vi.waitFor(() => expect(harness.tools.requestsOf('save_bytes')).toHaveLength(1));

    expect(harness.tools.requestsOf('save_bytes')[0]).toEqual({
      request: { text: 'AP8Q', reading: null },
      path: 'C:/out/decoded.bin',
    });
    await vi.waitFor(() => expect(TestBed.inject(StatusNotifier).status()?.key).toBe('tools.base64.saved'));
  });
});
