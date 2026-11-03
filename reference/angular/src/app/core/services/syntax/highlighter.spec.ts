import { beforeAll, describe, expect, it } from 'vitest';
import type { Language } from '@codemirror/language';
import { LANGUAGE_LABELS, LanguageTag } from '@core/model/language.model';
import { loadGrammar } from './grammars';
import { highlightLines, highlightRanges } from './highlighter';

/** Most languages colour a string, a number or a comment in this; the others say what they do. */
const ANY_CODE = 'x = "text" 42 // note\n# note';
const SAMPLES: Partial<Record<LanguageTag, string>> = {
  yml: 'key: value',
  xml: '<a>x</a>',
  html: '<p>hi</p>',
  vue: '<template><p>{{ x }}</p></template>',
  angular: '<p *ngIf="x">hi</p>',
  diff: '--- a/x\n+++ b/x\n-old\n+new',
  http: 'GET /api HTTP/1.1\nHost: x',
  tex: '\\section{Title}',
  liquid: '{% if x %}<p>{{ x }}</p>{% endif %}',
  mbox: 'From ada@example.com\nSubject: Hello',
  asc: '-----BEGIN PGP MESSAGE-----\n\nhQEMA\n-----END PGP MESSAGE-----',
  textile: 'h1. Title\n\n*bold*',
  tiki: '!Title\n__bold__',
  troff: '.TH TITLE 1\n.SH NAME',
};

async function grammar(tag: LanguageTag): Promise<Language | null> {
  return (await loadGrammar(tag))?.language ?? null;
}

describe('highlightLines', () => {
  let json: Language | null;
  beforeAll(async () => {
    json = await grammar('json');
  });

  it('escapes markup when there is no grammar', () => {
    expect(highlightLines('<b> & </b>', null)).toEqual(['&lt;b&gt; &amp; &lt;/b&gt;']);
  });

  it('produces one entry per source line', () => {
    expect(highlightLines('a\nb\nc', null)).toHaveLength(3);
    expect(highlightLines('{\n"a": 1\n}', json)).toHaveLength(3);
    expect(highlightLines('one\n', json)).toEqual(['one', '']);
  });

  it('colours each token in a span of its class, the text escaped inside', () => {
    expect(highlightLines('{"a": "<b>"}', json)).toEqual([
      '{<span class="tok-property">"a"</span>: <span class="tok-string">"&lt;b&gt;"</span>}',
    ]);
  });

  /** A comment over three lines is three lines, each closed where it ends. */
  it('cuts a token that spans lines at each line', async () => {
    const lines = highlightLines('/* one\ntwo */ x', await grammar('js'));

    expect(lines).toEqual([
      '<span class="tok-comment">/* one</span>',
      '<span class="tok-comment">two */</span> <span class="tok-property">x</span>',
    ]);
  });

  it.each((Object.keys(LANGUAGE_LABELS) as LanguageTag[]).filter((tag) => tag !== 'txt'))(
    'colours %s',
    async (tag) => {
      const lines = highlightLines(SAMPLES[tag] ?? ANY_CODE, await grammar(tag));

      expect(lines.join('\n')).toContain('tok-');
    },
  );

  it('leaves free text alone, which has nothing to colour', async () => {
    expect(await grammar('txt')).toBeNull();
  });
});

describe('highlightRanges', () => {
  let sql: Language | null;
  beforeAll(async () => {
    sql = await grammar('sql');
  });

  function textOf(content: string, classes: string): string[] {
    return highlightRanges(content, sql)
      .filter((range) => range.classes === classes)
      .map((range) => content.slice(range.from, range.to));
  }

  it('finds the coloured words at their offsets in the text', () => {
    const content = "SELECT id FROM notes WHERE title = 'a < b';";

    expect(textOf(content, 'tok-keyword')).toEqual(['SELECT', 'FROM', 'WHERE', '=']);
    expect(textOf(content, 'tok-string')).toEqual(["'a < b'"]);
  });

  it('colours nothing without a grammar', () => {
    expect(highlightRanges('SELECT 1', null)).toEqual([]);
  });

  it('keeps every range inside the text', async () => {
    const content = 'const a = "x & y";\n// done';
    const ranges = highlightRanges(content, await grammar('js'));

    expect(ranges.length).toBeGreaterThan(0);
    expect(
      ranges.every((range) => range.from >= 0 && range.to <= content.length && range.from < range.to),
    ).toBe(true);
  });
});

/** Ours: a few dozen lines each, so what each colours is said here. */
describe('the modes of ours', () => {
  async function classOf(tag: LanguageTag, content: string, text: string): Promise<string | undefined> {
    const ranges = highlightRanges(content, await grammar(tag));
    return ranges.find((range) => content.slice(range.from, range.to) === text)?.classes;
  }

  it.each([
    [
      'tf',
      'resource "aws_s3_bucket" "b" {\n  count = 2 # two\n}',
      [
        ['resource', 'tok-keyword'],
        ['"aws_s3_bucket"', 'tok-string'],
        ['2', 'tok-literal'],
        ['# two', 'tok-comment'],
      ],
    ],
    [
      'zig',
      'const std = @import("std");\nvar x: u8 = 0; // zero',
      [
        ['const', 'tok-keyword'],
        ['@import', 'tok-meta'],
        ['u8', 'tok-keyword'],
        ['// zero', 'tok-comment'],
      ],
    ],
    [
      'nix',
      "{ pkgs }: with pkgs; ''\n  multi\n'' # c",
      [
        ['with', 'tok-keyword'],
        ["''", 'tok-string'],
        ['# c', 'tok-comment'],
      ],
    ],
    [
      'prisma',
      'model Note {\n  id String @id @default(cuid())\n}',
      [
        ['model', 'tok-keyword'],
        ['String', 'tok-keyword'],
        ['@id', 'tok-meta'],
      ],
    ],
    [
      'sol',
      'contract C {\n  uint256 public n = 1; /* c */\n}',
      [
        ['contract', 'tok-keyword'],
        ['uint256', 'tok-keyword'],
        ['1', 'tok-literal'],
        ['/* c */', 'tok-comment'],
      ],
    ],
  ] as const)('colours %s', async (tag, content, expected) => {
    for (const [text, classes] of expected) {
      expect(await classOf(tag, content, text), text).toBe(classes);
    }
  });

  it('carries a block comment and a string over the lines they span', async () => {
    const content = 'contract C {\n  /* one\n  two */\n  string s = "a\\"b";\n}';
    const ranges = highlightRanges(content, await grammar('sol'));
    const comments = ranges.filter((range) => range.classes === 'tok-comment');

    expect(comments.map((range) => content.slice(range.from, range.to))).toEqual(['/* one', '  two */']);
    expect(ranges.some((range) => content.slice(range.from, range.to) === '"a\\"b"')).toBe(true);
  });
});
