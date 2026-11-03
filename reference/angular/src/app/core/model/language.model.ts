import type { Language } from '@core/ipc/bindings';

/** Generated from the Rust `Language` enum, so a variant added there breaks the build here. */
export type LanguageTag = Language;

/** Six hues by family: the label names the language, the colour only groups it at a glance. */
export type LanguageHue = 'blue' | 'amber' | 'green' | 'purple' | 'red' | 'neutral';

interface LanguageInfo {
  /** The badge's, and the menu's. */
  readonly label: string;
  /** What it is called in full, in a sentence. */
  readonly name: string;
  readonly hue: LanguageHue;
}

/**
 * The key order is the menu's: the common languages with their kin first, the rest by name,
 * Markdown and plain text last. Blue is typed and read before it runs, amber what a page runs,
 * green what a machine runs, purple data and what shapes it, red configuration, neutral prose.
 */
export const LANGUAGES: Readonly<Record<LanguageTag, LanguageInfo>> = {
  json: { label: 'JSON', name: 'JSON', hue: 'blue' },
  js: { label: 'JS', name: 'JavaScript', hue: 'amber' },
  jsx: { label: 'JSX', name: 'JSX', hue: 'amber' },
  ts: { label: 'TS', name: 'TypeScript', hue: 'blue' },
  tsx: { label: 'TSX', name: 'TSX', hue: 'blue' },
  py: { label: 'PY', name: 'Python', hue: 'green' },
  rs: { label: 'RS', name: 'Rust', hue: 'green' },
  go: { label: 'GO', name: 'Go', hue: 'green' },
  java: { label: 'JAVA', name: 'Java', hue: 'blue' },
  cs: { label: 'CS', name: 'C#', hue: 'blue' },
  php: { label: 'PHP', name: 'PHP', hue: 'amber' },
  c: { label: 'C', name: 'C', hue: 'green' },
  cpp: { label: 'C++', name: 'C++', hue: 'green' },
  sql: { label: 'SQL', name: 'SQL', hue: 'purple' },
  graphql: { label: 'GRAPHQL', name: 'GraphQL', hue: 'purple' },
  yml: { label: 'YML', name: 'YAML', hue: 'red' },
  toml: { label: 'TOML', name: 'TOML', hue: 'red' },
  xml: { label: 'XML', name: 'XML', hue: 'blue' },
  html: { label: 'HTML', name: 'HTML', hue: 'amber' },
  vue: { label: 'VUE', name: 'Vue', hue: 'amber' },
  angular: { label: 'NG', name: 'Angular template', hue: 'amber' },
  css: { label: 'CSS', name: 'CSS', hue: 'purple' },
  scss: { label: 'SCSS', name: 'SCSS', hue: 'purple' },
  less: { label: 'LESS', name: 'Less', hue: 'purple' },
  sass: { label: 'SASS', name: 'Sass', hue: 'purple' },
  sh: { label: 'SH', name: 'Shell', hue: 'green' },
  ps1: { label: 'PS1', name: 'PowerShell', hue: 'green' },
  kt: { label: 'KT', name: 'Kotlin', hue: 'green' },
  rb: { label: 'RB', name: 'Ruby', hue: 'green' },
  swift: { label: 'SWIFT', name: 'Swift', hue: 'green' },
  dart: { label: 'DART', name: 'Dart', hue: 'green' },
  ex: { label: 'EX', name: 'Elixir', hue: 'green' },
  apl: { label: 'APL', name: 'APL', hue: 'green' },
  armasm: { label: 'ARM', name: 'ARM assembly', hue: 'green' },
  asn1: { label: 'ASN.1', name: 'ASN.1', hue: 'blue' },
  asm: { label: 'ASM', name: 'Assembly (GAS)', hue: 'green' },
  asterisk: { label: 'ASTERISK', name: 'Asterisk dialplan', hue: 'red' },
  bf: { label: 'BF', name: 'Brainfuck', hue: 'green' },
  ceylon: { label: 'CEYLON', name: 'Ceylon', hue: 'blue' },
  clj: { label: 'CLJ', name: 'Clojure', hue: 'green' },
  gss: { label: 'GSS', name: 'Closure Stylesheets', hue: 'purple' },
  cmake: { label: 'CMAKE', name: 'CMake', hue: 'red' },
  cobol: { label: 'COBOL', name: 'COBOL', hue: 'green' },
  coffee: { label: 'COFFEE', name: 'CoffeeScript', hue: 'amber' },
  lisp: { label: 'LISP', name: 'Common Lisp', hue: 'green' },
  cr: { label: 'CR', name: 'Crystal', hue: 'green' },
  cypher: { label: 'CYPHER', name: 'Cypher', hue: 'purple' },
  cython: { label: 'CYTHON', name: 'Cython', hue: 'green' },
  d: { label: 'D', name: 'D', hue: 'green' },
  diff: { label: 'DIFF', name: 'Diff', hue: 'neutral' },
  dockerfile: { label: 'DOCKER', name: 'Dockerfile', hue: 'red' },
  dtd: { label: 'DTD', name: 'DTD', hue: 'blue' },
  dylan: { label: 'DYLAN', name: 'Dylan', hue: 'green' },
  ebnf: { label: 'EBNF', name: 'EBNF', hue: 'purple' },
  ecl: { label: 'ECL', name: 'ECL', hue: 'purple' },
  eiffel: { label: 'EIFFEL', name: 'Eiffel', hue: 'blue' },
  elm: { label: 'ELM', name: 'Elm', hue: 'amber' },
  erl: { label: 'ERL', name: 'Erlang', hue: 'green' },
  fs: { label: 'F#', name: 'F#', hue: 'blue' },
  factor: { label: 'FACTOR', name: 'Factor', hue: 'green' },
  fcl: { label: 'FCL', name: 'FCL', hue: 'green' },
  forth: { label: 'FORTH', name: 'Forth', hue: 'green' },
  fortran: { label: 'FORTRAN', name: 'Fortran', hue: 'green' },
  gherkin: { label: 'GHERKIN', name: 'Gherkin', hue: 'neutral' },
  glsl: { label: 'GLSL', name: 'GLSL', hue: 'green' },
  groovy: { label: 'GROOVY', name: 'Groovy', hue: 'green' },
  hs: { label: 'HS', name: 'Haskell', hue: 'blue' },
  haxe: { label: 'HAXE', name: 'Haxe', hue: 'blue' },
  http: { label: 'HTTP', name: 'HTTP', hue: 'amber' },
  hxml: { label: 'HXML', name: 'HXML', hue: 'red' },
  idl: { label: 'IDL', name: 'IDL', hue: 'blue' },
  ini: { label: 'INI', name: 'INI', hue: 'red' },
  jinja: { label: 'JINJA', name: 'Jinja', hue: 'amber' },
  jsonld: { label: 'JSON-LD', name: 'JSON-LD', hue: 'blue' },
  jl: { label: 'JL', name: 'Julia', hue: 'green' },
  tex: { label: 'TEX', name: 'LaTeX', hue: 'neutral' },
  liquid: { label: 'LIQUID', name: 'Liquid', hue: 'amber' },
  ls: { label: 'LS', name: 'LiveScript', hue: 'amber' },
  lua: { label: 'LUA', name: 'Lua', hue: 'green' },
  mathematica: { label: 'WL', name: 'Mathematica', hue: 'green' },
  matlab: { label: 'MATLAB', name: 'MATLAB / Octave', hue: 'green' },
  mbox: { label: 'MBOX', name: 'mbox', hue: 'neutral' },
  mirc: { label: 'MIRC', name: 'mIRC', hue: 'green' },
  modelica: { label: 'MODELICA', name: 'Modelica', hue: 'blue' },
  mscgen: { label: 'MSC', name: 'Mscgen', hue: 'purple' },
  mumps: { label: 'MUMPS', name: 'MUMPS', hue: 'green' },
  ntriples: { label: 'NT', name: 'N-Triples', hue: 'purple' },
  nesc: { label: 'NESC', name: 'nesC', hue: 'green' },
  nginx: { label: 'NGINX', name: 'Nginx', hue: 'red' },
  nix: { label: 'NIX', name: 'Nix', hue: 'red' },
  nsis: { label: 'NSIS', name: 'NSIS', hue: 'red' },
  objc: { label: 'OBJC', name: 'Objective-C', hue: 'green' },
  objcpp: { label: 'OBJC++', name: 'Objective-C++', hue: 'green' },
  ml: { label: 'ML', name: 'OCaml', hue: 'blue' },
  oz: { label: 'OZ', name: 'Oz', hue: 'green' },
  pascal: { label: 'PASCAL', name: 'Pascal', hue: 'green' },
  pegjs: { label: 'PEGJS', name: 'PEG.js', hue: 'purple' },
  perl: { label: 'PERL', name: 'Perl', hue: 'green' },
  asc: { label: 'PGP', name: 'PGP armor', hue: 'neutral' },
  pig: { label: 'PIG', name: 'Pig', hue: 'purple' },
  prisma: { label: 'PRISMA', name: 'Prisma', hue: 'purple' },
  properties: { label: 'PROPS', name: 'Properties', hue: 'red' },
  proto: { label: 'PROTO', name: 'Protocol Buffers', hue: 'blue' },
  pug: { label: 'PUG', name: 'Pug', hue: 'amber' },
  puppet: { label: 'PUPPET', name: 'Puppet', hue: 'red' },
  q: { label: 'Q', name: 'Q', hue: 'purple' },
  r: { label: 'R', name: 'R', hue: 'purple' },
  rpmspec: { label: 'RPM', name: 'RPM spec', hue: 'red' },
  sas: { label: 'SAS', name: 'SAS', hue: 'purple' },
  scala: { label: 'SCALA', name: 'Scala', hue: 'blue' },
  scheme: { label: 'SCHEME', name: 'Scheme', hue: 'green' },
  sieve: { label: 'SIEVE', name: 'Sieve', hue: 'red' },
  smalltalk: { label: 'ST', name: 'Smalltalk', hue: 'green' },
  sol: { label: 'SOL', name: 'Solidity', hue: 'green' },
  solr: { label: 'SOLR', name: 'Solr', hue: 'purple' },
  sparql: { label: 'SPARQL', name: 'SPARQL', hue: 'purple' },
  spreadsheet: { label: 'SHEET', name: 'Spreadsheet formula', hue: 'purple' },
  squirrel: { label: 'NUT', name: 'Squirrel', hue: 'green' },
  sml: { label: 'SML', name: 'Standard ML', hue: 'blue' },
  styl: { label: 'STYL', name: 'Stylus', hue: 'purple' },
  tcl: { label: 'TCL', name: 'Tcl', hue: 'green' },
  tf: { label: 'TF', name: 'Terraform / HCL', hue: 'red' },
  textile: { label: 'TEXTILE', name: 'Textile', hue: 'neutral' },
  tiddlywiki: { label: 'TW', name: 'TiddlyWiki', hue: 'neutral' },
  tiki: { label: 'TIKI', name: 'Tiki wiki', hue: 'neutral' },
  troff: { label: 'TROFF', name: 'troff', hue: 'neutral' },
  ttcn: { label: 'TTCN', name: 'TTCN-3', hue: 'blue' },
  ttcncfg: { label: 'TTCN-CFG', name: 'TTCN-3 configuration', hue: 'red' },
  turtle: { label: 'TTL', name: 'Turtle', hue: 'purple' },
  vbs: { label: 'VBS', name: 'VBScript', hue: 'green' },
  velocity: { label: 'VTL', name: 'Velocity', hue: 'amber' },
  verilog: { label: 'VERILOG', name: 'Verilog', hue: 'green' },
  vhdl: { label: 'VHDL', name: 'VHDL', hue: 'green' },
  vb: { label: 'VB', name: 'Visual Basic', hue: 'blue' },
  webidl: { label: 'WEBIDL', name: 'Web IDL', hue: 'blue' },
  wat: { label: 'WAT', name: 'WebAssembly text', hue: 'amber' },
  xquery: { label: 'XQUERY', name: 'XQuery', hue: 'purple' },
  yacas: { label: 'YACAS', name: 'Yacas', hue: 'green' },
  z80: { label: 'Z80', name: 'Z80 assembly', hue: 'green' },
  zig: { label: 'ZIG', name: 'Zig', hue: 'green' },
  md: { label: 'MD', name: 'Markdown', hue: 'neutral' },
  txt: { label: 'TXT', name: 'Text', hue: 'neutral' },
};

function each<T>(read: (info: LanguageInfo) => T): Readonly<Record<LanguageTag, T>> {
  return Object.fromEntries(
    (Object.entries(LANGUAGES) as [LanguageTag, LanguageInfo][]).map(([tag, info]) => [tag, read(info)]),
  ) as Record<LanguageTag, T>;
}

export const LANGUAGE_LABELS = each((info) => info.label);
export const LANGUAGE_FULL_NAMES = each((info) => info.name);
export const LANGUAGE_HUES = each((info) => info.hue);

/**
 * The full name where the label is an abbreviation of something else — « YML », YAML — which a
 * menu shows as a hint and a filter also matches. « TXT » keeps its label alone.
 */
export const LANGUAGE_NAMES: Partial<Record<LanguageTag, string>> = Object.fromEntries(
  (Object.entries(LANGUAGE_FULL_NAMES) as [LanguageTag, string][]).filter(
    ([id, name]) => id !== 'txt' && name.toUpperCase() !== LANGUAGE_LABELS[id],
  ),
);

/** Kept when the value received is not (or no longer) recognised. */
export const FALLBACK_LANGUAGE: LanguageTag = 'txt';

/** Narrows a free string — a `<select>`'s value, never data from the bridge. */
export function isLanguageTag(value: unknown): value is LanguageTag {
  return typeof value === 'string' && Object.hasOwn(LANGUAGES, value);
}
