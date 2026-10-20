import { WritableSignal, signal } from '@angular/core';
import type { Language, LanguageSupport, StreamParser } from '@codemirror/language';
import type { LanguageTag } from '@core/model/language.model';

/** A legacy mode, or one of ours, for the languages Lezer has no grammar of. */
async function stream(parser: Promise<StreamParser<unknown>>): Promise<LanguageSupport> {
  const { LanguageSupport, StreamLanguage } = await import('@codemirror/language');
  return new LanguageSupport(StreamLanguage.define(advancing(await parser)));
}

/**
 * ⚠️ CodeMirror throws when a mode has not moved after ten tries, and some legacy modes stall
 * on ordinary text (Ceylon on a plain string): a card would draw nothing. One character is
 * left plain instead.
 */
function advancing<State>(parser: StreamParser<State>): StreamParser<State> {
  return {
    ...parser,
    token(stream, state) {
      for (let tries = 0; tries < 9; tries++) {
        const style = parser.token(stream, state);
        if (stream.pos > stream.start) return style;
      }
      stream.next();
      return null;
    },
  };
}

/**
 * One grammar per language, for the editor and the cards alike: each its own chunk, loaded the
 * first time something in it is drawn. Plain text has none. A snippet is PHP from its first
 * line, not a page with `<?php` islands in it; GraphQL's package is its grammar alone, its
 * completion and lint wanting a schema a snippet does not have.
 */
export const GRAMMARS: Readonly<Record<LanguageTag, (() => Promise<LanguageSupport>) | null>> = {
  angular: () => import('@codemirror/lang-angular').then((m) => m.angular()),
  apl: () => stream(import('@codemirror/legacy-modes/mode/apl').then((m) => m.apl)),
  armasm: () => stream(import('@codemirror/legacy-modes/mode/gas').then((m) => m.gasArm)),
  asc: () => stream(import('@codemirror/legacy-modes/mode/asciiarmor').then((m) => m.asciiArmor)),
  asm: () => stream(import('@codemirror/legacy-modes/mode/gas').then((m) => m.gas)),
  asn1: () => stream(import('@codemirror/legacy-modes/mode/asn1').then((m) => m.asn1({}))),
  asterisk: () => stream(import('@codemirror/legacy-modes/mode/asterisk').then((m) => m.asterisk)),
  bf: () => stream(import('@codemirror/legacy-modes/mode/brainfuck').then((m) => m.brainfuck)),
  c: () => import('@codemirror/lang-cpp').then((m) => m.cpp()),
  ceylon: () => stream(import('@codemirror/legacy-modes/mode/clike').then((m) => m.ceylon)),
  clj: () => stream(import('@codemirror/legacy-modes/mode/clojure').then((m) => m.clojure)),
  cmake: () => stream(import('@codemirror/legacy-modes/mode/cmake').then((m) => m.cmake)),
  cobol: () => stream(import('@codemirror/legacy-modes/mode/cobol').then((m) => m.cobol)),
  coffee: () => stream(import('@codemirror/legacy-modes/mode/coffeescript').then((m) => m.coffeeScript)),
  cpp: () => import('@codemirror/lang-cpp').then((m) => m.cpp()),
  cr: () => stream(import('@codemirror/legacy-modes/mode/crystal').then((m) => m.crystal)),
  cs: () => stream(import('@codemirror/legacy-modes/mode/clike').then((m) => m.csharp)),
  css: () => import('@codemirror/lang-css').then((m) => m.css()),
  cypher: () => stream(import('@codemirror/legacy-modes/mode/cypher').then((m) => m.cypher)),
  cython: () => stream(import('@codemirror/legacy-modes/mode/python').then((m) => m.cython)),
  d: () => stream(import('@codemirror/legacy-modes/mode/d').then((m) => m.d)),
  dart: () => stream(import('@codemirror/legacy-modes/mode/clike').then((m) => m.dart)),
  diff: () => stream(import('@codemirror/legacy-modes/mode/diff').then((m) => m.diff)),
  dockerfile: () => stream(import('@codemirror/legacy-modes/mode/dockerfile').then((m) => m.dockerFile)),
  dtd: () => stream(import('@codemirror/legacy-modes/mode/dtd').then((m) => m.dtd)),
  dylan: () => stream(import('@codemirror/legacy-modes/mode/dylan').then((m) => m.dylan)),
  ebnf: () => stream(import('@codemirror/legacy-modes/mode/ebnf').then((m) => m.ebnf)),
  ecl: () => stream(import('@codemirror/legacy-modes/mode/ecl').then((m) => m.ecl)),
  eiffel: () => stream(import('@codemirror/legacy-modes/mode/eiffel').then((m) => m.eiffel)),
  elm: () => stream(import('@codemirror/legacy-modes/mode/elm').then((m) => m.elm)),
  erl: () => stream(import('@codemirror/legacy-modes/mode/erlang').then((m) => m.erlang)),
  ex: () => import('codemirror-lang-elixir').then((m) => m.elixir()),
  factor: () => stream(import('@codemirror/legacy-modes/mode/factor').then((m) => m.factor)),
  fcl: () => stream(import('@codemirror/legacy-modes/mode/fcl').then((m) => m.fcl)),
  forth: () => stream(import('@codemirror/legacy-modes/mode/forth').then((m) => m.forth)),
  fortran: () => stream(import('@codemirror/legacy-modes/mode/fortran').then((m) => m.fortran)),
  fs: () => stream(import('@codemirror/legacy-modes/mode/mllike').then((m) => m.fSharp)),
  gherkin: () => stream(import('@codemirror/legacy-modes/mode/gherkin').then((m) => m.gherkin)),
  glsl: () => stream(import('@codemirror/legacy-modes/mode/clike').then((m) => m.shader)),
  go: () => import('@codemirror/lang-go').then((m) => m.go()),
  graphql: () => import('cm6-graphql').then((m) => m.graphqlLanguageSupport()),
  groovy: () => stream(import('@codemirror/legacy-modes/mode/groovy').then((m) => m.groovy)),
  gss: () => stream(import('@codemirror/legacy-modes/mode/css').then((m) => m.gss)),
  haxe: () => stream(import('@codemirror/legacy-modes/mode/haxe').then((m) => m.haxe)),
  hs: () => stream(import('@codemirror/legacy-modes/mode/haskell').then((m) => m.haskell)),
  html: () => import('@codemirror/lang-html').then((m) => m.html()),
  http: () => stream(import('@codemirror/legacy-modes/mode/http').then((m) => m.http)),
  hxml: () => stream(import('@codemirror/legacy-modes/mode/haxe').then((m) => m.hxml)),
  idl: () => stream(import('@codemirror/legacy-modes/mode/idl').then((m) => m.idl)),
  ini: () => stream(import('@codemirror/legacy-modes/mode/properties').then((m) => m.properties)),
  java: () => import('@codemirror/lang-java').then((m) => m.java()),
  jinja: () => stream(import('@codemirror/legacy-modes/mode/jinja2').then((m) => m.jinja2)),
  jl: () => stream(import('@codemirror/legacy-modes/mode/julia').then((m) => m.julia)),
  js: () => import('@codemirror/lang-javascript').then((m) => m.javascript()),
  json: () => import('@codemirror/lang-json').then((m) => m.json()),
  jsonld: () => import('@codemirror/lang-json').then((m) => m.json()),
  jsx: () => import('@codemirror/lang-javascript').then((m) => m.javascript({ jsx: true })),
  kt: () => stream(import('@codemirror/legacy-modes/mode/clike').then((m) => m.kotlin)),
  less: () => stream(import('@codemirror/legacy-modes/mode/css').then((m) => m.less)),
  liquid: () => import('@codemirror/lang-liquid').then((m) => m.liquid()),
  lisp: () => stream(import('@codemirror/legacy-modes/mode/commonlisp').then((m) => m.commonLisp)),
  ls: () => stream(import('@codemirror/legacy-modes/mode/livescript').then((m) => m.liveScript)),
  lua: () => stream(import('@codemirror/legacy-modes/mode/lua').then((m) => m.lua)),
  mathematica: () => stream(import('@codemirror/legacy-modes/mode/mathematica').then((m) => m.mathematica)),
  matlab: () => stream(import('@codemirror/legacy-modes/mode/octave').then((m) => m.octave)),
  mbox: () => stream(import('@codemirror/legacy-modes/mode/mbox').then((m) => m.mbox)),
  md: () => import('@codemirror/lang-markdown').then((m) => m.markdown()),
  mirc: () => stream(import('@codemirror/legacy-modes/mode/mirc').then((m) => m.mirc)),
  ml: () => stream(import('@codemirror/legacy-modes/mode/mllike').then((m) => m.oCaml)),
  modelica: () => stream(import('@codemirror/legacy-modes/mode/modelica').then((m) => m.modelica)),
  mscgen: () => stream(import('@codemirror/legacy-modes/mode/mscgen').then((m) => m.mscgen)),
  mumps: () => stream(import('@codemirror/legacy-modes/mode/mumps').then((m) => m.mumps)),
  nesc: () => stream(import('@codemirror/legacy-modes/mode/clike').then((m) => m.nesC)),
  nginx: () => stream(import('@codemirror/legacy-modes/mode/nginx').then((m) => m.nginx)),
  nix: () => stream(import('./modes/nix').then((m) => m.nix)),
  nsis: () => stream(import('@codemirror/legacy-modes/mode/nsis').then((m) => m.nsis)),
  ntriples: () => stream(import('@codemirror/legacy-modes/mode/ntriples').then((m) => m.ntriples)),
  objc: () => stream(import('@codemirror/legacy-modes/mode/clike').then((m) => m.objectiveC)),
  objcpp: () => stream(import('@codemirror/legacy-modes/mode/clike').then((m) => m.objectiveCpp)),
  oz: () => stream(import('@codemirror/legacy-modes/mode/oz').then((m) => m.oz)),
  pascal: () => stream(import('@codemirror/legacy-modes/mode/pascal').then((m) => m.pascal)),
  pegjs: () => stream(import('@codemirror/legacy-modes/mode/pegjs').then((m) => m.pegjs)),
  perl: () => stream(import('@codemirror/legacy-modes/mode/perl').then((m) => m.perl)),
  php: () => import('@codemirror/lang-php').then((m) => m.php({ plain: true })),
  pig: () => stream(import('@codemirror/legacy-modes/mode/pig').then((m) => m.pig)),
  prisma: () => stream(import('./modes/prisma').then((m) => m.prisma)),
  properties: () => stream(import('@codemirror/legacy-modes/mode/properties').then((m) => m.properties)),
  proto: () => stream(import('@codemirror/legacy-modes/mode/protobuf').then((m) => m.protobuf)),
  ps1: () => stream(import('@codemirror/legacy-modes/mode/powershell').then((m) => m.powerShell)),
  pug: () => stream(import('@codemirror/legacy-modes/mode/pug').then((m) => m.pug)),
  puppet: () => stream(import('@codemirror/legacy-modes/mode/puppet').then((m) => m.puppet)),
  py: () => import('@codemirror/lang-python').then((m) => m.python()),
  q: () => stream(import('@codemirror/legacy-modes/mode/q').then((m) => m.q)),
  r: () => stream(import('@codemirror/legacy-modes/mode/r').then((m) => m.r)),
  rb: () => stream(import('@codemirror/legacy-modes/mode/ruby').then((m) => m.ruby)),
  rpmspec: () => stream(import('@codemirror/legacy-modes/mode/rpm').then((m) => m.rpmSpec)),
  rs: () => import('@codemirror/lang-rust').then((m) => m.rust()),
  sas: () => stream(import('@codemirror/legacy-modes/mode/sas').then((m) => m.sas)),
  sass: () => import('@codemirror/lang-sass').then((m) => m.sass({ indented: true })),
  scala: () => stream(import('@codemirror/legacy-modes/mode/clike').then((m) => m.scala)),
  scheme: () => stream(import('@codemirror/legacy-modes/mode/scheme').then((m) => m.scheme)),
  scss: () => import('@codemirror/lang-sass').then((m) => m.sass()),
  sh: () => stream(import('@codemirror/legacy-modes/mode/shell').then((m) => m.shell)),
  sieve: () => stream(import('@codemirror/legacy-modes/mode/sieve').then((m) => m.sieve)),
  smalltalk: () => stream(import('@codemirror/legacy-modes/mode/smalltalk').then((m) => m.smalltalk)),
  sml: () => stream(import('@codemirror/legacy-modes/mode/mllike').then((m) => m.sml)),
  sol: () => stream(import('./modes/solidity').then((m) => m.solidity)),
  solr: () => stream(import('@codemirror/legacy-modes/mode/solr').then((m) => m.solr)),
  sparql: () => stream(import('@codemirror/legacy-modes/mode/sparql').then((m) => m.sparql)),
  spreadsheet: () => stream(import('@codemirror/legacy-modes/mode/spreadsheet').then((m) => m.spreadsheet)),
  sql: () => import('@codemirror/lang-sql').then((m) => m.sql()),
  squirrel: () => stream(import('@codemirror/legacy-modes/mode/clike').then((m) => m.squirrel)),
  styl: () => stream(import('@codemirror/legacy-modes/mode/stylus').then((m) => m.stylus)),
  swift: () => stream(import('@codemirror/legacy-modes/mode/swift').then((m) => m.swift)),
  tcl: () => stream(import('@codemirror/legacy-modes/mode/tcl').then((m) => m.tcl)),
  tex: () => stream(import('@codemirror/legacy-modes/mode/stex').then((m) => m.stex)),
  textile: () => stream(import('@codemirror/legacy-modes/mode/textile').then((m) => m.textile)),
  tf: () => stream(import('./modes/hcl').then((m) => m.hcl)),
  tiddlywiki: () => stream(import('@codemirror/legacy-modes/mode/tiddlywiki').then((m) => m.tiddlyWiki)),
  tiki: () => stream(import('@codemirror/legacy-modes/mode/tiki').then((m) => m.tiki)),
  toml: () => stream(import('@codemirror/legacy-modes/mode/toml').then((m) => m.toml)),
  troff: () => stream(import('@codemirror/legacy-modes/mode/troff').then((m) => m.troff)),
  ts: () => import('@codemirror/lang-javascript').then((m) => m.javascript({ typescript: true })),
  tsx: () => import('@codemirror/lang-javascript').then((m) => m.javascript({ jsx: true, typescript: true })),
  ttcn: () => stream(import('@codemirror/legacy-modes/mode/ttcn').then((m) => m.ttcn)),
  ttcncfg: () => stream(import('@codemirror/legacy-modes/mode/ttcn-cfg').then((m) => m.ttcnCfg)),
  turtle: () => stream(import('@codemirror/legacy-modes/mode/turtle').then((m) => m.turtle)),
  txt: null,
  vb: () => stream(import('@codemirror/legacy-modes/mode/vb').then((m) => m.vb)),
  vbs: () => stream(import('@codemirror/legacy-modes/mode/vbscript').then((m) => m.vbScript)),
  velocity: () => stream(import('@codemirror/legacy-modes/mode/velocity').then((m) => m.velocity)),
  verilog: () => stream(import('@codemirror/legacy-modes/mode/verilog').then((m) => m.verilog)),
  vhdl: () => stream(import('@codemirror/legacy-modes/mode/vhdl').then((m) => m.vhdl)),
  vue: () => import('@codemirror/lang-vue').then((m) => m.vue()),
  wat: () => stream(import('@codemirror/legacy-modes/mode/wast').then((m) => m.wast)),
  webidl: () => stream(import('@codemirror/legacy-modes/mode/webidl').then((m) => m.webIDL)),
  xml: () => import('@codemirror/lang-xml').then((m) => m.xml()),
  xquery: () => stream(import('@codemirror/legacy-modes/mode/xquery').then((m) => m.xQuery)),
  yacas: () => stream(import('@codemirror/legacy-modes/mode/yacas').then((m) => m.yacas)),
  yml: () => import('@codemirror/lang-yaml').then((m) => m.yaml()),
  z80: () => stream(import('@codemirror/legacy-modes/mode/z80').then((m) => m.z80)),
  zig: () => stream(import('./modes/zig').then((m) => m.zig)),
};

const requested = new Map<LanguageTag, Promise<LanguageSupport | null>>();
const arrived = new Map<LanguageTag, WritableSignal<Language | null>>();

/** The language's grammar, loaded once whoever asks first; nothing for plain text. */
export function loadGrammar(tag: LanguageTag): Promise<LanguageSupport | null> {
  let pending = requested.get(tag);
  if (!pending) {
    pending = (GRAMMARS[tag]?.() ?? Promise.resolve(null)).catch(() => null);
    requested.set(tag, pending);
  }
  return pending;
}

/**
 * The grammar if it has arrived, asking for it otherwise: read in a `computed`, it runs again
 * once the grammar is there — only those of that language — and draws plain text until then.
 */
export function grammarOf(tag: LanguageTag): Language | null {
  let grammar = arrived.get(tag);
  if (!grammar) {
    const slot = signal<Language | null>(null);
    arrived.set(tag, slot);
    void loadGrammar(tag).then((support) => slot.set(support?.language ?? null));
    grammar = slot;
  }
  return grammar();
}
