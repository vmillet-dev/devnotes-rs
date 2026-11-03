import { describe, expect, it } from 'vitest';
import { shieldFields, unshieldFields } from './field-shield';

describe('shieldFields', () => {
  it('swaps every {{…}} for an identifier of its own length', () => {
    const shielded = shieldFields('host = {{db_host}}; port = {{ port = 5432 }}', 0)!;

    expect(shielded.text).not.toContain('{{');
    expect(shielded.text.length).toBe('host = {{db_host}}; port = {{ port = 5432 }}'.length);
    expect(shielded.swaps.map((swap) => swap.original)).toEqual(['{{db_host}}', '{{ port = 5432 }}']);
    expect(shielded.swaps.every((swap) => /^[a-z0-9]+$/.test(swap.standIn))).toBe(true);
  });

  it('takes a marker the text does not already contain', () => {
    const shielded = shieldFields('qz zq {{a}}', 0)!;

    expect(shielded.marker).not.toBe('qz');
    expect('qz zq'.includes(shielded.marker)).toBe(false);
  });

  it('leaves an unterminated {{ as it is', () => {
    expect(shieldFields('a {{ b', 0)!.text).toBe('a {{ b');
  });

  it('lengthens a token too short to hold its stand-in', () => {
    const shielded = shieldFields('{{}}', 0)!;

    expect(shielded.text.length).toBeGreaterThan(4);
  });

  it('moves the cursor with the text around it', () => {
    // After a token longer than its stand-in, inside one, and before any.
    expect(shieldFields('{{}} ab', 6)!.cursor).toBe(6 + shieldFields('{{}}', 0)!.text.length - 4);
    expect(shieldFields('ab {{field}} cd', 5)!.cursor).toBe(3);
    expect(shieldFields('ab {{field}}', 1)!.cursor).toBe(1);
  });

  it('gives up when no marker is free', () => {
    const letters = 'qzjkvw';
    const every = [...letters].flatMap((a) => [...letters].map((b) => a + b)).join(' ');

    expect(shieldFields(every, 0)).toBeNull();
  });
});

describe('unshieldFields', () => {
  it('puts every field back, the cursor with them', () => {
    const shielded = shieldFields('a({{x}},{{y}})', 0)!;
    const formatted = `a(\n  ${shielded.swaps[0]!.standIn},\n  ${shielded.swaps[1]!.standIn},\n);\n`;

    expect(unshieldFields(formatted, formatted.length, shielded)).toEqual({
      text: 'a(\n  {{x}},\n  {{y}},\n);\n',
      cursor: 'a(\n  {{x}},\n  {{y}},\n);\n'.length,
    });
  });

  it('refuses a text that lost a field', () => {
    const shielded = shieldFields('{{x}} {{y}}', 0)!;

    expect(unshieldFields(shielded.swaps[0]!.standIn, 0, shielded)).toBeNull();
  });

  it('refuses a text that copied one', () => {
    const shielded = shieldFields('{{x}}', 0)!;
    const standIn = shielded.swaps[0]!.standIn;

    expect(unshieldFields(`${standIn} ${standIn}`, 0, shielded)).toBeNull();
  });

  it('refuses fields that changed places', () => {
    const shielded = shieldFields('{{x}} {{y}}', 0)!;
    const [x, y] = shielded.swaps;

    expect(unshieldFields(`${y!.standIn} ${x!.standIn}`, 0, shielded)).toBeNull();
  });
});
