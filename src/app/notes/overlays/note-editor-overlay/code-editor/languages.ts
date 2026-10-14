import type { Extension } from '@codemirror/state';
import type { StreamParser } from '@codemirror/language';
import type { LanguageTag } from '@core/model/language.model';

/** A legacy mode, for the languages Lezer has no grammar of. */
async function stream(parser: Promise<StreamParser<unknown>>): Promise<Extension> {
  const { StreamLanguage } = await import('@codemirror/language');
  return StreamLanguage.define(await parser);
}

/**
 * Each language its own chunk, loaded the first time a snippet of it opens: the grammars of
 * twenty-six languages are not the editor's weight. Plain text has none.
 */
export const LANGUAGES: Readonly<Record<LanguageTag, (() => Promise<Extension>) | null>> = {
  json: () => import('@codemirror/lang-json').then((m) => m.json()),
  js: () => import('@codemirror/lang-javascript').then((m) => m.javascript()),
  ts: () => import('@codemirror/lang-javascript').then((m) => m.javascript({ typescript: true })),
  py: () => import('@codemirror/lang-python').then((m) => m.python()),
  rs: () => import('@codemirror/lang-rust').then((m) => m.rust()),
  go: () => import('@codemirror/lang-go').then((m) => m.go()),
  java: () => import('@codemirror/lang-java').then((m) => m.java()),
  cs: () => stream(import('@codemirror/legacy-modes/mode/clike').then((m) => m.csharp)),
  // A snippet is PHP from its first line, not a page with `<?php` islands in it.
  php: () => import('@codemirror/lang-php').then((m) => m.php({ plain: true })),
  c: () => import('@codemirror/lang-cpp').then((m) => m.cpp()),
  sql: () => import('@codemirror/lang-sql').then((m) => m.sql()),
  // The grammar alone: the package's completion and lint want a schema a snippet does not have.
  graphql: () => import('cm6-graphql').then((m) => m.graphqlLanguageSupport()),
  yml: () => import('@codemirror/lang-yaml').then((m) => m.yaml()),
  toml: () => stream(import('@codemirror/legacy-modes/mode/toml').then((m) => m.toml)),
  xml: () => import('@codemirror/lang-xml').then((m) => m.xml()),
  html: () => import('@codemirror/lang-html').then((m) => m.html()),
  css: () => import('@codemirror/lang-css').then((m) => m.css()),
  scss: () => import('@codemirror/lang-sass').then((m) => m.sass()),
  sh: () => stream(import('@codemirror/legacy-modes/mode/shell').then((m) => m.shell)),
  ps1: () => stream(import('@codemirror/legacy-modes/mode/powershell').then((m) => m.powerShell)),
  kt: () => stream(import('@codemirror/legacy-modes/mode/clike').then((m) => m.kotlin)),
  rb: () => stream(import('@codemirror/legacy-modes/mode/ruby').then((m) => m.ruby)),
  swift: () => stream(import('@codemirror/legacy-modes/mode/swift').then((m) => m.swift)),
  dart: () => stream(import('@codemirror/legacy-modes/mode/clike').then((m) => m.dart)),
  ex: () => import('codemirror-lang-elixir').then((m) => m.elixir()),
  md: () => import('@codemirror/lang-markdown').then((m) => m.markdown()),
  txt: null,
};

/** The language's grammar, or nothing for plain text. */
export async function languageOf(tag: LanguageTag): Promise<Extension> {
  return (await LANGUAGES[tag]?.()) ?? [];
}
