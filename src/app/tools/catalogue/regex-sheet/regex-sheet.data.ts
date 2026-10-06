export const REGEX_GROUPS = ['anchors', 'classes', 'quantifiers', 'groups', 'flags', 'escapes'] as const;
export type RegexGroup = (typeof REGEX_GROUPS)[number];

/** `note`: supported, with a condition the words spell out — a flag it needs, a limit. */
export type Support = 'yes' | 'no' | 'note';

export interface RegexEntry {
  /** As it is written, and what the row copies. Unique: a flag is its letter. */
  readonly syntax: string;
  readonly group: RegexGroup;
  readonly js: Support;
  readonly pcre: Support;
  /** A pattern, a text, and what it matches there; `␊` stands for a line break. */
  readonly example: { readonly pattern: string; readonly text: string; readonly matches: readonly string[] };
}

export interface RegexWords {
  readonly entries: Readonly<
    Record<string, { readonly what: string; readonly js?: string; readonly pcre?: string }>
  >;
}

const BOTH = { js: 'yes', pcre: 'yes' } as const;
const PCRE_ONLY = { js: 'no', pcre: 'yes' } as const;
const JS_ONLY = { js: 'yes', pcre: 'no' } as const;

export const REGEX_ENTRIES: readonly RegexEntry[] = [
  { syntax: '^', group: 'anchors', ...BOTH, example: { pattern: '^Bon', text: 'Bonjour', matches: ['Bon'] } },
  { syntax: '$', group: 'anchors', ...BOTH, example: { pattern: '\\d+$', text: 'lot 42', matches: ['42'] } },
  {
    syntax: '\\b',
    group: 'anchors',
    ...BOTH,
    example: { pattern: '\\bcat\\b', text: 'cat scatter', matches: ['cat'] },
  },
  {
    syntax: '\\B',
    group: 'anchors',
    ...BOTH,
    example: { pattern: '\\Bcat', text: 'cat scatter', matches: ['cat'] },
  },
  {
    syntax: '\\A',
    group: 'anchors',
    ...PCRE_ONLY,
    example: { pattern: '\\Aab', text: 'abab', matches: ['ab'] },
  },
  {
    syntax: '\\z',
    group: 'anchors',
    ...PCRE_ONLY,
    example: { pattern: 'ab\\z', text: 'abab', matches: ['ab'] },
  },
  {
    syntax: '\\Z',
    group: 'anchors',
    ...PCRE_ONLY,
    example: { pattern: 'fin\\Z', text: 'fin␊', matches: ['fin'] },
  },
  {
    syntax: '\\G',
    group: 'anchors',
    ...PCRE_ONLY,
    example: { pattern: '\\G\\d', text: '12a3', matches: ['1', '2'] },
  },

  {
    syntax: '.',
    group: 'classes',
    ...BOTH,
    example: { pattern: 'a.c', text: 'abc a-c', matches: ['abc', 'a-c'] },
  },
  {
    syntax: '\\d',
    group: 'classes',
    ...BOTH,
    example: { pattern: '\\d+', text: 'v12.3', matches: ['12', '3'] },
  },
  { syntax: '\\D', group: 'classes', ...BOTH, example: { pattern: '\\D+', text: 'v12', matches: ['v'] } },
  {
    syntax: '\\w',
    group: 'classes',
    ...BOTH,
    example: { pattern: '\\w+', text: 'user_id-2', matches: ['user_id', '2'] },
  },
  { syntax: '\\W', group: 'classes', ...BOTH, example: { pattern: '\\W', text: 'a-b', matches: ['-'] } },
  { syntax: '\\s', group: 'classes', ...BOTH, example: { pattern: 'a\\sb', text: 'a b', matches: ['a b'] } },
  {
    syntax: '\\S',
    group: 'classes',
    ...BOTH,
    example: { pattern: '\\S+', text: 'x = 1', matches: ['x', '=', '1'] },
  },
  {
    syntax: '[abc]',
    group: 'classes',
    ...BOTH,
    example: { pattern: '[aeiou]', text: 'regex', matches: ['e', 'e'] },
  },
  {
    syntax: '[^abc]',
    group: 'classes',
    ...BOTH,
    example: { pattern: '[^0-9]+', text: 'a1b2', matches: ['a', 'b'] },
  },
  {
    syntax: '[a-z]',
    group: 'classes',
    ...BOTH,
    example: { pattern: '[a-f0-9]+', text: '0xff', matches: ['0', 'ff'] },
  },
  {
    syntax: '\\p{L}',
    group: 'classes',
    js: 'note',
    pcre: 'yes',
    example: { pattern: '\\p{L}+', text: 'été 2026', matches: ['été'] },
  },
  {
    syntax: '\\P{L}',
    group: 'classes',
    js: 'note',
    pcre: 'yes',
    example: { pattern: '\\P{L}+', text: 'été 2026', matches: [' 2026'] },
  },
  {
    syntax: '[[:alpha:]]',
    group: 'classes',
    ...PCRE_ONLY,
    example: { pattern: '[[:alpha:]]+', text: 'abc123', matches: ['abc'] },
  },
  {
    syntax: '\\h',
    group: 'classes',
    ...PCRE_ONLY,
    example: { pattern: 'a\\hb', text: 'a b', matches: ['a b'] },
  },
  {
    syntax: '\\R',
    group: 'classes',
    ...PCRE_ONLY,
    example: { pattern: 'a\\Rb', text: 'a␍␊b', matches: ['a␍␊b'] },
  },
  { syntax: '\\X', group: 'classes', ...PCRE_ONLY, example: { pattern: '^\\X$', text: 'é', matches: ['é'] } },

  {
    syntax: '*',
    group: 'quantifiers',
    ...BOTH,
    example: { pattern: 'ab*', text: 'a ab abbb', matches: ['a', 'ab', 'abbb'] },
  },
  {
    syntax: '+',
    group: 'quantifiers',
    ...BOTH,
    example: { pattern: 'ab+', text: 'a ab abbb', matches: ['ab', 'abbb'] },
  },
  {
    syntax: '?',
    group: 'quantifiers',
    ...BOTH,
    example: { pattern: 'colou?r', text: 'color colour', matches: ['color', 'colour'] },
  },
  {
    syntax: '{n}',
    group: 'quantifiers',
    ...BOTH,
    example: { pattern: '\\d{4}', text: '2026-10', matches: ['2026'] },
  },
  {
    syntax: '{n,}',
    group: 'quantifiers',
    ...BOTH,
    example: { pattern: '\\d{2,}', text: '1 22 333', matches: ['22', '333'] },
  },
  {
    syntax: '{n,m}',
    group: 'quantifiers',
    ...BOTH,
    example: { pattern: '\\d{2,3}', text: '1 22 4444', matches: ['22', '444'] },
  },
  {
    syntax: '*?',
    group: 'quantifiers',
    ...BOTH,
    example: { pattern: '<.+?>', text: '<a><b>', matches: ['<a>', '<b>'] },
  },
  {
    syntax: '*+',
    group: 'quantifiers',
    ...PCRE_ONLY,
    example: { pattern: '\\d++\\d', text: '123', matches: [] },
  },

  {
    syntax: '(…)',
    group: 'groups',
    ...BOTH,
    example: { pattern: '(\\d+)-(\\d+)', text: '10-20', matches: ['10-20'] },
  },
  {
    syntax: '(?:…)',
    group: 'groups',
    ...BOTH,
    example: { pattern: '(?:ab)+', text: 'ababx', matches: ['abab'] },
  },
  {
    syntax: '(?<name>…)',
    group: 'groups',
    ...BOTH,
    example: { pattern: '(?<year>\\d{4})', text: 'en 2026', matches: ['2026'] },
  },
  {
    syntax: '(?P<name>…)',
    group: 'groups',
    ...PCRE_ONLY,
    example: { pattern: '(?P<year>\\d{4})', text: 'en 2026', matches: ['2026'] },
  },
  {
    syntax: '\\1',
    group: 'groups',
    ...BOTH,
    example: { pattern: '(\\w)\\1', text: 'hello', matches: ['ll'] },
  },
  {
    syntax: '\\k<name>',
    group: 'groups',
    ...BOTH,
    example: { pattern: '(?<c>\\w)\\k<c>', text: 'hello', matches: ['ll'] },
  },
  {
    syntax: 'a|b',
    group: 'groups',
    ...BOTH,
    example: { pattern: 'cat|dog', text: 'a dog', matches: ['dog'] },
  },
  {
    syntax: '(?=…)',
    group: 'groups',
    ...BOTH,
    example: { pattern: '\\d+(?=€)', text: '12€ 30$', matches: ['12'] },
  },
  {
    syntax: '(?!…)',
    group: 'groups',
    ...BOTH,
    example: { pattern: '\\b\\d+\\b(?!€)', text: '12€ 30$', matches: ['30'] },
  },
  {
    syntax: '(?<=…)',
    group: 'groups',
    js: 'yes',
    pcre: 'note',
    example: { pattern: '(?<=\\$)\\d+', text: '€12 $30', matches: ['30'] },
  },
  {
    syntax: '(?<!…)',
    group: 'groups',
    js: 'yes',
    pcre: 'note',
    example: { pattern: '(?<!\\$)\\b\\d+', text: '$12 30', matches: ['30'] },
  },
  {
    syntax: '(?>…)',
    group: 'groups',
    ...PCRE_ONLY,
    example: { pattern: '(?>a+)b', text: 'aaab', matches: ['aaab'] },
  },
  {
    syntax: '(?i:…)',
    group: 'groups',
    js: 'note',
    pcre: 'yes',
    example: { pattern: '(?i:abc)D', text: 'ABCD abcd', matches: ['ABCD'] },
  },
  {
    syntax: '(?i)',
    group: 'groups',
    ...PCRE_ONLY,
    example: { pattern: '(?i)abc', text: 'ABC', matches: ['ABC'] },
  },
  {
    syntax: '(?R)',
    group: 'groups',
    ...PCRE_ONLY,
    example: { pattern: '\\((?:[^()]|(?R))*\\)', text: 'f((a)(b))', matches: ['((a)(b))'] },
  },
  {
    syntax: '(?#…)',
    group: 'groups',
    ...PCRE_ONLY,
    example: { pattern: 'a(?#comment)b', text: 'ab', matches: ['ab'] },
  },

  { syntax: 'i', group: 'flags', ...BOTH, example: { pattern: '/abc/i', text: 'ABC', matches: ['ABC'] } },
  { syntax: 'm', group: 'flags', ...BOTH, example: { pattern: '/^\\d/m', text: 'a␊1', matches: ['1'] } },
  { syntax: 's', group: 'flags', ...BOTH, example: { pattern: '/a.b/s', text: 'a␊b', matches: ['a␊b'] } },
  {
    syntax: 'g',
    group: 'flags',
    ...JS_ONLY,
    example: { pattern: '/\\d/g', text: 'a1b2', matches: ['1', '2'] },
  },
  {
    syntax: 'u',
    group: 'flags',
    js: 'yes',
    pcre: 'note',
    example: { pattern: '/^.$/u', text: '😀', matches: ['😀'] },
  },
  { syntax: 'y', group: 'flags', ...JS_ONLY, example: { pattern: '/\\d/y', text: '1a2', matches: ['1'] } },
  { syntax: 'd', group: 'flags', ...JS_ONLY, example: { pattern: '/b(c)/d', text: 'abc', matches: ['bc'] } },
  {
    syntax: 'v',
    group: 'flags',
    ...JS_ONLY,
    example: { pattern: '/[\\p{L}--[a-z]]/v', text: 'aÉb', matches: ['É'] },
  },
  {
    syntax: 'x',
    group: 'flags',
    ...PCRE_ONLY,
    example: { pattern: '(?x) \\d+  # digits', text: 'n° 42', matches: ['42'] },
  },
  {
    syntax: 'n',
    group: 'flags',
    ...PCRE_ONLY,
    example: { pattern: '(?n)(a)(?<b>b)', text: 'ab', matches: ['ab'] },
  },
  {
    syntax: 'U',
    group: 'flags',
    ...PCRE_ONLY,
    example: { pattern: '(?U)a+', text: 'aaa', matches: ['a', 'a', 'a'] },
  },

  {
    syntax: '\\.',
    group: 'escapes',
    ...BOTH,
    example: { pattern: '\\d\\.\\d', text: 'v3.5', matches: ['3.5'] },
  },
  { syntax: '\\t', group: 'escapes', ...BOTH, example: { pattern: 'a\\tb', text: 'a⇥b', matches: ['a⇥b'] } },
  { syntax: '\\n', group: 'escapes', ...BOTH, example: { pattern: 'a\\nb', text: 'a␊b', matches: ['a␊b'] } },
  { syntax: '\\xhh', group: 'escapes', ...BOTH, example: { pattern: '\\x41', text: 'ABC', matches: ['A'] } },
  {
    syntax: '\\uhhhh',
    group: 'escapes',
    ...JS_ONLY,
    example: { pattern: '\\u00e9', text: 'café', matches: ['é'] },
  },
  {
    syntax: '\\u{h…}',
    group: 'escapes',
    js: 'note',
    pcre: 'no',
    example: { pattern: '/\\u{1F600}/u', text: '😀', matches: ['😀'] },
  },
  {
    syntax: '\\x{h…}',
    group: 'escapes',
    ...PCRE_ONLY,
    example: { pattern: '\\x{1F600}', text: '😀', matches: ['😀'] },
  },
  {
    syntax: '\\Q…\\E',
    group: 'escapes',
    ...PCRE_ONLY,
    example: { pattern: '\\Q1+1=2\\E', text: '1+1=2', matches: ['1+1=2'] },
  },
  { syntax: '\\cX', group: 'escapes', ...BOTH, example: { pattern: '\\cJ', text: 'a␊b', matches: ['␊'] } },
];
