use crate::closed_enum::closed_enum;

use super::model::{Note, NoteDraft, NotePatch};

closed_enum! {
    /// Closed: the front receives a generated union, so an unknown value stops compiling
    /// there instead of being refused at runtime.
    pub enum Language {
        Json = "json",
        Js = "js",
        Jsx = "jsx",
        Ts = "ts",
        Tsx = "tsx",
        Py = "py",
        Rs = "rs",
        Go = "go",
        Java = "java",
        Cs = "cs",
        Php = "php",
        C = "c",
        Cpp = "cpp",
        Sql = "sql",
        Graphql = "graphql",
        Yml = "yml",
        Toml = "toml",
        Xml = "xml",
        Html = "html",
        Vue = "vue",
        Angular = "angular",
        Css = "css",
        Scss = "scss",
        Less = "less",
        Sass = "sass",
        Sh = "sh",
        /// PowerShell: what generated code is saved as, and what a pasted cmdlet reads as.
        Ps1 = "ps1",
        Kt = "kt",
        Rb = "rb",
        Swift = "swift",
        Dart = "dart",
        Ex = "ex",
        Apl = "apl",
        Armasm = "armasm",
        Asn1 = "asn1",
        Asm = "asm",
        Asterisk = "asterisk",
        Bf = "bf",
        Ceylon = "ceylon",
        Clj = "clj",
        Gss = "gss",
        Cmake = "cmake",
        Cobol = "cobol",
        Coffee = "coffee",
        Lisp = "lisp",
        Cr = "cr",
        Cypher = "cypher",
        Cython = "cython",
        D = "d",
        Diff = "diff",
        Dockerfile = "dockerfile",
        Dtd = "dtd",
        Dylan = "dylan",
        Ebnf = "ebnf",
        Ecl = "ecl",
        Eiffel = "eiffel",
        Elm = "elm",
        Erl = "erl",
        Fs = "fs",
        Factor = "factor",
        Fcl = "fcl",
        Forth = "forth",
        Fortran = "fortran",
        Gherkin = "gherkin",
        Glsl = "glsl",
        Groovy = "groovy",
        Hs = "hs",
        Haxe = "haxe",
        Http = "http",
        Hxml = "hxml",
        Idl = "idl",
        Ini = "ini",
        Jinja = "jinja",
        Jsonld = "jsonld",
        Jl = "jl",
        Tex = "tex",
        Liquid = "liquid",
        Ls = "ls",
        Lua = "lua",
        Mathematica = "mathematica",
        Matlab = "matlab",
        Mbox = "mbox",
        Mirc = "mirc",
        Modelica = "modelica",
        Mscgen = "mscgen",
        Mumps = "mumps",
        Ntriples = "ntriples",
        Nesc = "nesc",
        Nginx = "nginx",
        Nix = "nix",
        Nsis = "nsis",
        Objc = "objc",
        Objcpp = "objcpp",
        Ml = "ml",
        Oz = "oz",
        Pascal = "pascal",
        Pegjs = "pegjs",
        Perl = "perl",
        Asc = "asc",
        Pig = "pig",
        Prisma = "prisma",
        Properties = "properties",
        Proto = "proto",
        Pug = "pug",
        Puppet = "puppet",
        Q = "q",
        R = "r",
        Rpmspec = "rpmspec",
        Sas = "sas",
        Scala = "scala",
        Scheme = "scheme",
        Sieve = "sieve",
        Smalltalk = "smalltalk",
        Sol = "sol",
        Solr = "solr",
        Sparql = "sparql",
        Spreadsheet = "spreadsheet",
        Squirrel = "squirrel",
        Sml = "sml",
        Styl = "styl",
        Tcl = "tcl",
        Tf = "tf",
        Textile = "textile",
        Tiddlywiki = "tiddlywiki",
        Tiki = "tiki",
        Troff = "troff",
        Ttcn = "ttcn",
        Ttcncfg = "ttcncfg",
        Turtle = "turtle",
        Vbs = "vbs",
        Velocity = "velocity",
        Verilog = "verilog",
        Vhdl = "vhdl",
        Vb = "vb",
        Webidl = "webidl",
        Wat = "wat",
        Xquery = "xquery",
        Yacas = "yacas",
        Z80 = "z80",
        Zig = "zig",
        Md = "md",
        /// Default, and the signal that the front end chose nothing.
        #[default]
        Txt = "txt",
    }
}

/// The front end's choice if it made one, `txt` meaning it made none.
pub fn for_draft(draft: &NoteDraft) -> Language {
    if draft.language == Language::default() {
        from_content(&draft.content)
    } else {
        draft.language
    }
}

/// A snippet's first content, when nothing was chosen: creation sees no content if the title
/// came first, the paste after. The refusals keep detection from becoming a correction.
pub fn after_patch(before: &Note, patch: &NotePatch) -> Option<Language> {
    if patch.language.is_some()
        || before.language != Language::default()
        || !before.content.trim().is_empty()
    {
        return None;
    }

    let content = patch.content.as_ref()?;
    if content.trim().is_empty() {
        return None;
    }

    Some(from_content(content))
}

/// What one marker is worth. Three tiers and no more: a marker declares how much it proves,
/// and ten steps would bring back an ordering nobody can read.
const SIGNATURE: u32 = 6;
const STRONG: u32 = 3;
const WEAK: u32 = 1;

/// Below this, the highest score is not an answer: one weak marker is not enough, two are.
/// A wrong language colours the body, badges the card and files the note under a facet;
/// `txt` says nothing, which is honest. A tie is no answer either.
const MIN_CONFIDENCE: u32 = 2;

/// The content in the shapes the markers read it in, computed once.
struct Sample<'a> {
    trimmed: &'a str,
    lower: String,
    lines: Vec<&'a str>,
}

impl<'a> Sample<'a> {
    fn of(content: &'a str) -> Self {
        let trimmed = content.trim();
        Self {
            trimmed,
            lower: trimmed.to_lowercase(),
            lines: trimmed.lines().map(str::trim).collect(),
        }
    }

    fn has(&self, needle: &str) -> bool {
        self.trimmed.contains(needle)
    }

    fn any_line(&self, predicate: impl Fn(&str) -> bool) -> bool {
        self.lines.iter().copied().any(predicate)
    }

    fn line_starts_with_any(&self, prefixes: &[&str]) -> bool {
        self.any_line(|line| starts_with_any(line, prefixes))
    }
}

/// Adds a weight when a marker is present, which is the whole of the arithmetic.
fn worth(weight: u32, present: bool) -> u32 {
    if present { weight } else { 0 }
}

/// How much one language believes a sample is its own.
type Scorer = fn(&Sample<'_>) -> u32;

/// Every language that can be guessed at, each scoring itself. The order means nothing: a
/// language added here needs markers with honest weights, not a slot.
const SCORERS: [(Language, Scorer); 76] = [
    (Language::Json, score_json),
    (Language::Php, score_php),
    (Language::Xml, score_xml),
    (Language::Html, score_html),
    (Language::Sql, score_sql),
    (Language::Graphql, score_graphql),
    (Language::Toml, score_toml),
    (Language::Py, score_python),
    (Language::Go, score_go),
    (Language::Rs, score_rust),
    (Language::Java, score_java),
    (Language::Cs, score_csharp),
    (Language::C, score_c),
    (Language::Ts, score_typescript),
    (Language::Js, score_javascript),
    (Language::Css, score_css),
    (Language::Scss, score_scss),
    (Language::Yml, score_yaml),
    (Language::Md, score_markdown),
    (Language::Sh, score_shell),
    (Language::Ps1, score_powershell),
    (Language::Kt, score_kotlin),
    (Language::Rb, score_ruby),
    (Language::Swift, score_swift),
    (Language::Dart, score_dart),
    (Language::Ex, score_elixir),
    (Language::Cpp, score_cpp),
    (Language::Jsx, score_jsx),
    (Language::Tsx, score_tsx),
    (Language::Vue, score_vue),
    (Language::Angular, score_angular),
    (Language::Less, score_less),
    (Language::Dockerfile, score_dockerfile),
    (Language::Tf, score_hcl),
    (Language::Nginx, score_nginx),
    (Language::Proto, score_proto),
    (Language::Lua, score_lua),
    (Language::Hs, score_haskell),
    (Language::Ml, score_ocaml),
    (Language::Clj, score_clojure),
    (Language::Lisp, score_lisp),
    (Language::Scheme, score_scheme),
    (Language::R, score_r),
    (Language::Cmake, score_cmake),
    (Language::Erl, score_erlang),
    (Language::Elm, score_elm),
    (Language::Perl, score_perl),
    (Language::Groovy, score_groovy),
    (Language::Verilog, score_verilog),
    (Language::Vhdl, score_vhdl),
    (Language::Diff, score_diff),
    (Language::Http, score_http),
    (Language::Tex, score_tex),
    (Language::Gherkin, score_gherkin),
    (Language::Scala, score_scala),
    (Language::Objc, score_objc),
    (Language::Glsl, score_glsl),
    (Language::Cobol, score_cobol),
    (Language::Fortran, score_fortran),
    (Language::Sparql, score_sparql),
    (Language::Cypher, score_cypher),
    (Language::Zig, score_zig),
    (Language::Nix, score_nix),
    (Language::Prisma, score_prisma),
    (Language::Sol, score_solidity),
    (Language::Asc, score_armor),
    (Language::Jinja, score_jinja),
    (Language::Liquid, score_liquid),
    (Language::Wat, score_wat),
    (Language::Jsonld, score_jsonld),
    (Language::Matlab, score_matlab),
    (Language::D, score_d),
    (Language::Vb, score_vb),
    (Language::Tcl, score_tcl),
    (Language::Cython, score_cython),
    (Language::Puppet, score_puppet),
];

/// The languages detection can answer; the others are only ever chosen.
pub fn detectable() -> impl Iterator<Item = Language> {
    SCORERS.iter().map(|(language, _)| *language)
}

/// The best-scoring language, or `txt` when nothing proved itself. Weights settle what an
/// order would: `<?php` outweighs the `<` of a processing instruction, and Rust's `println!`
/// outweighs JavaScript's `=>`.
pub fn from_content(content: &str) -> Language {
    let sample = Sample::of(content);
    if sample.trimmed.is_empty() {
        return Language::default();
    }
    // A shebang settles the question on its own, whatever follows it.
    if sample.trimmed.starts_with("#!") {
        return from_shebang(sample.lines.first().copied().unwrap_or_default());
    }

    let mut best = (Language::default(), 0);
    let mut runner_up = 0;
    for (language, score) in SCORERS {
        let scored = score(&sample);
        if scored > best.1 {
            runner_up = best.1;
            best = (language, scored);
        } else if scored > runner_up {
            runner_up = scored;
        }
    }

    if best.1 < MIN_CONFIDENCE || best.1 == runner_up {
        return Language::default();
    }
    best.0
}

fn starts_with_any(line: &str, prefixes: &[&str]) -> bool {
    prefixes.iter().any(|prefix| line.starts_with(prefix))
}

/// Structural, and worth a signature: nothing else here is a quoted object or an array from
/// its first character to its last.
fn score_json(sample: &Sample<'_>) -> u32 {
    let content = sample.trimmed;
    let wrapped = (content.starts_with('{') && content.ends_with('}'))
        || (content.starts_with('[') && content.ends_with(']'));

    worth(
        SIGNATURE,
        wrapped && (content.contains('"') || content.starts_with('[')),
    )
}

/// A signature, which keeps it ahead of the markup score its `<` also earns.
fn score_php(sample: &Sample<'_>) -> u32 {
    worth(SIGNATURE, sample.lower.starts_with("<?php")) + worth(WEAK, sample.has("->"))
}

/// The generic shape is XML's alone, and HTML scores only what is its own: sharing it made
/// the two tie on a plain `<config>…</config>`, and a tie is `txt`.
fn score_xml(sample: &Sample<'_>) -> u32 {
    let looks_like_markup =
        sample.lower.starts_with('<') && sample.trimmed.ends_with('>') && sample.has("</");

    worth(STRONG, looks_like_markup) + worth(SIGNATURE, sample.lower.starts_with("<?xml"))
}

/// Its tags are a signature: they must outweigh the markup shape XML earns on the same text.
fn score_html(sample: &Sample<'_>) -> u32 {
    const TAGS: [&str; 8] = [
        "<div", "<span", "<p>", "<body", "<head", "<a ", "<ul", "<table",
    ];

    let doctype = sample.lower.starts_with("<!doctype html") || sample.lower.starts_with("<html");

    worth(SIGNATURE, doctype) + worth(SIGNATURE, TAGS.iter().any(|tag| sample.lower.contains(tag)))
}

fn score_sql(sample: &Sample<'_>) -> u32 {
    const STATEMENTS: [&str; 8] = [
        "select ",
        "insert into",
        "update ",
        "delete from",
        "create table",
        "alter table",
        "drop table",
        "with ",
    ];

    worth(STRONG, starts_with_any(&sample.lower, &STATEMENTS))
        + worth(
            WEAK,
            sample.lower.contains(" from ") || sample.lower.contains(" where "),
        )
}

/// An operation or a fragment is a signature; a schema's `type User {` is TypeScript's shape
/// too, so it counts only with a non-null type, which TypeScript never writes after a name.
fn score_graphql(sample: &Sample<'_>) -> u32 {
    const NON_NULL: [&str; 6] = ["ID!", "String!", "Int!", "Boolean!", "Float!", "]!"];

    let operation = sample.has("{")
        && sample.any_line(|line| {
            ["query", "mutation", "subscription"].iter().any(|keyword| {
                line.strip_prefix(keyword)
                    .is_some_and(|rest| rest.starts_with([' ', '{', '(']))
            })
        });
    let fragment = sample.has("... on ")
        || sample.any_line(|line| line.starts_with("fragment ") && line.contains(" on "));
    let definition = sample.any_line(|line| {
        let Some(name) = ["type ", "input ", "interface ", "enum "]
            .iter()
            .find_map(|keyword| line.strip_prefix(keyword))
        else {
            return false;
        };
        name.ends_with('{')
            && !name.contains('=')
            && name.chars().next().is_some_and(char::is_uppercase)
    });

    worth(SIGNATURE, operation || fragment)
        + worth(STRONG, NON_NULL.iter().any(|marker| sample.has(marker)))
        + worth(WEAK, definition)
}

/// A section and an assignment: `[…]` alone could be an array on its own line.
fn score_toml(sample: &Sample<'_>) -> u32 {
    worth(
        SIGNATURE,
        sample.any_line(is_toml_section) && sample.any_line(is_assignment),
    )
}

fn is_toml_section(line: &str) -> bool {
    let Some(inner) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) else {
        return false;
    };
    let inner = inner.trim_matches(|c| c == '[' || c == ']');

    !inner.is_empty() && inner.chars().all(is_identifier_char)
}

fn is_assignment(line: &str) -> bool {
    line.split_once('=').is_some_and(|(key, _)| {
        let key = key.trim();
        !key.is_empty() && !key.contains(char::is_whitespace)
    })
}

fn is_identifier_char(c: char) -> bool {
    c.is_alphanumeric() || matches!(c, '_' | '-' | '.')
}

/// `class` needs its trailing colon: without it, this is TypeScript's.
fn score_python(sample: &Sample<'_>) -> u32 {
    worth(SIGNATURE, sample.has("__name__"))
        + worth(
            STRONG,
            sample.line_starts_with_any(&["def ", "async def ", "elif "]),
        )
        + worth(
            STRONG,
            sample.any_line(|line| line.starts_with("class ") && line.ends_with(':')),
        )
        + worth(
            STRONG,
            sample.any_line(|line| line.starts_with("from ") && line.contains(" import ")),
        )
        + worth(WEAK, sample.has("self."))
}

/// `fmt.Print` and `package main` are Go's alone; `import (` is its grouped form.
fn score_go(sample: &Sample<'_>) -> u32 {
    worth(SIGNATURE, sample.has("fmt.Print"))
        + worth(
            STRONG,
            sample.line_starts_with_any(&["func ", "package main", "import ("]),
        )
        + worth(WEAK, sample.has(":=") || sample.has("err != nil"))
}

/// `struct`, `trait` and `enum` count, weakly: a marker Rust shares with TypeScript is worth
/// little, not nothing.
fn score_rust(sample: &Sample<'_>) -> u32 {
    worth(SIGNATURE, sample.has("println!") || sample.has("let mut "))
        + worth(
            STRONG,
            sample.any_line(|line| {
                let line = line.strip_prefix("pub ").unwrap_or(line);
                starts_with_any(line, &["fn ", "async fn ", "impl "])
            }),
        )
        + worth(
            STRONG,
            sample.any_line(|line| line.starts_with("use ") && line.contains("::")),
        )
        + worth(
            WEAK,
            sample.any_line(|line| {
                let line = line.strip_prefix("pub ").unwrap_or(line);
                starts_with_any(line, &["struct ", "trait ", "enum "])
            }),
        )
        + worth(
            WEAK,
            sample.has("&str") || sample.has("Vec<") || sample.has("Option<"),
        )
}

/// `public class` is left to neither this nor C#: both write it.
fn score_java(sample: &Sample<'_>) -> u32 {
    worth(
        SIGNATURE,
        sample.has("System.out.print") || sample.has("public static void main"),
    ) + worth(
        STRONG,
        sample.any_line(|line| line.starts_with("import java")),
    )
}

fn score_csharp(sample: &Sample<'_>) -> u32 {
    worth(SIGNATURE, sample.has("Console.Write"))
        + worth(
            STRONG,
            sample.line_starts_with_any(&["using System", "namespace "]),
        )
}

/// `#include` is the one marker nothing else here writes.
fn score_c(sample: &Sample<'_>) -> u32 {
    worth(
        SIGNATURE,
        sample.any_line(|line| line.starts_with("#include")),
    ) + worth(STRONG, sample.has("int main(") && sample.has("printf("))
}

/// Its own markers only, never JavaScript's: inheriting them would tie the two on every file.
fn score_typescript(sample: &Sample<'_>) -> u32 {
    const ANNOTATIONS: [&str; 4] = [": string", ": number", ": boolean", "implements "];
    const DECLARATIONS: [&str; 4] = ["interface ", "type ", "enum ", "declare "];

    worth(STRONG, ANNOTATIONS.iter().any(|marker| sample.has(marker)))
        + worth(
            STRONG,
            sample.any_line(|line| {
                let line = line.strip_prefix("export ").unwrap_or(line);
                starts_with_any(line, &DECLARATIONS)
            }),
        )
}

/// `=>` is the greediest marker in this file — C++, Kotlin, Swift, Scala, Dart and anything
/// with a lambda write it — so it is worth the least.
fn score_javascript(sample: &Sample<'_>) -> u32 {
    const KEYWORDS: [&str; 7] = [
        "function ",
        "const ",
        "let ",
        "var ",
        "export ",
        "import ",
        "class ",
    ];

    worth(
        SIGNATURE,
        sample.has("console.log") || sample.has("require("),
    ) + worth(WEAK, sample.has("=>"))
        + worth(WEAK, sample.line_starts_with_any(&KEYWORDS))
}

/// A selector and a declaration, plus something only a stylesheet writes: the shape alone is
/// strong evidence, not proof — `export interface Note {` and `id: string;` have it too.
fn score_css(sample: &Sample<'_>) -> u32 {
    if !sample.has("{") || !sample.has("}") {
        return 0;
    }

    let has_declaration = sample.any_line(|line| {
        line.find(':')
            .zip(line.find(';'))
            .is_some_and(|(colon, semicolon)| colon < semicolon)
    });
    let has_selector = sample.any_line(|line| {
        line.ends_with('{')
            && line.chars().next().is_some_and(|c| {
                c.is_ascii_alphabetic() || matches!(c, '.' | '#' | '@' | ':' | '*')
            })
    });
    let stylesheet_only = sample.has("--")
        || ["px;", "rem;", "%;", "px ", "em;", "vh;", "vw;", "fr;"]
            .iter()
            .any(|unit| sample.has(unit));

    worth(STRONG, has_declaration && has_selector) + worth(WEAK, stylesheet_only)
}

/// Its own markers only, never CSS's, which it earns as well: a stylesheet with no variable,
/// mixin or nesting is CSS, whatever file it came from.
fn score_scss(sample: &Sample<'_>) -> u32 {
    const AT_RULES: [&str; 6] = [
        "@mixin ",
        "@include ",
        "@use ",
        "@forward ",
        "@extend ",
        "@each ",
    ];

    let variable = sample.any_line(|line| {
        line.strip_prefix('$').is_some_and(|rest| {
            rest.split_once(':')
                .is_some_and(|(name, _)| !name.is_empty() && name.chars().all(is_identifier_char))
        })
    });

    // Native CSS nests too, but a nested stylesheet pasted today is far more often SCSS, and a
    // tie with CSS's own score would answer `txt`.
    let nested = sample.line_starts_with_any(&["&:", "&.", "&-", "& "]);

    worth(
        SIGNATURE,
        variable || nested || sample.line_starts_with_any(&AT_RULES),
    )
}

fn score_yaml(sample: &Sample<'_>) -> u32 {
    // These belong to languages that score on them instead.
    if sample.has(";") || sample.has("{") {
        return 0;
    }

    worth(SIGNATURE, sample.trimmed.starts_with("---"))
        + worth(STRONG, sample.any_line(is_mapping))
        + worth(WEAK, sample.any_line(|line| line.starts_with("- ")))
}

/// The space required after the colon rules out a URL, whose `http://…` would otherwise
/// read as a key.
fn is_mapping(line: &str) -> bool {
    let Some((key, value)) = line.split_once(':') else {
        return false;
    };

    !key.is_empty()
        && key.chars().all(is_identifier_char)
        && (value.is_empty() || value.starts_with(' '))
}

fn score_markdown(sample: &Sample<'_>) -> u32 {
    const LINE_MARKERS: [&str; 5] = ["# ", "## ", "### ", "* ", "> "];

    worth(SIGNATURE, sample.has("```"))
        + worth(STRONG, sample.has("]("))
        + worth(STRONG, sample.line_starts_with_any(&LINE_MARKERS))
}

/// Deliberately thin: a wide list would catch prose.
fn score_shell(sample: &Sample<'_>) -> u32 {
    const COMMANDS: [&str; 10] = [
        "echo ", "cd ", "ls ", "cat ", "grep ", "sudo ", "npm ", "git ", "docker ", "curl ",
    ];

    worth(STRONG, sample.line_starts_with_any(&COMMANDS))
        + worth(
            WEAK,
            sample.any_line(|line| line.starts_with('$') || line.starts_with("./")),
        )
        + worth(WEAK, sample.has(" | ") || sample.has(" && "))
}

/// A cmdlet's `Verb-Noun`, a variable assigned, a hashtable: a shell line has none of the three.
fn score_powershell(sample: &Sample<'_>) -> u32 {
    const CMDLETS: [&str; 8] = [
        "Invoke-RestMethod",
        "Invoke-WebRequest",
        "Get-ChildItem",
        "Get-Item",
        "Set-Location",
        "Write-Host",
        "Write-Output",
        "New-Item",
    ];

    worth(STRONG, CMDLETS.iter().any(|cmdlet| sample.has(cmdlet)))
        + worth(
            WEAK,
            sample.any_line(|line| line.starts_with('$') && line.contains(" = ")),
        )
        + worth(WEAK, sample.has("@{"))
}

/// `fun` and `val` begin a line in Kotlin alone: Swift writes `func`, Scala is not offered.
fn score_kotlin(sample: &Sample<'_>) -> u32 {
    worth(SIGNATURE, sample.has("fun main("))
        + worth(
            STRONG,
            sample.line_starts_with_any(&["fun ", "val ", "data class ", "object "]),
        )
        + worth(WEAK, sample.has("?.") && sample.has("val "))
}

/// A line that is only `end` is Ruby's or Elixir's: worth little, and the same to both.
fn score_ruby(sample: &Sample<'_>) -> u32 {
    worth(
        SIGNATURE,
        sample.line_starts_with_any(&["require '", "require \""]),
    ) + worth(
        STRONG,
        sample.line_starts_with_any(&["puts ", "attr_accessor "]),
    ) + worth(STRONG, sample.has(" do |"))
        + worth(WEAK, sample.any_line(|line| line == "end"))
}

fn score_swift(sample: &Sample<'_>) -> u32 {
    worth(
        SIGNATURE,
        sample.line_starts_with_any(&["import Foundation", "import SwiftUI", "import UIKit"]),
    ) + worth(STRONG, sample.has("guard let ") || sample.has("if let "))
        + worth(WEAK, sample.has("func ") && sample.has(" -> "))
}

fn score_dart(sample: &Sample<'_>) -> u32 {
    worth(
        SIGNATURE,
        sample.line_starts_with_any(&["import 'package:", "import \"package:"]),
    ) + worth(
        STRONG,
        sample.line_starts_with_any(&["Future<void> main(", "void main() {"]),
    ) + worth(WEAK, sample.line_starts_with_any(&["final "]))
}

fn score_elixir(sample: &Sample<'_>) -> u32 {
    worth(
        SIGNATURE,
        sample.line_starts_with_any(&["defmodule "]) || sample.has("|>"),
    ) + worth(STRONG, sample.has("IO.puts") || sample.has("IO.inspect"))
        + worth(WEAK, sample.any_line(|line| line == "end"))
}

/// The interpreter a shebang names, `env` or not; a shell, or one this file does not know, is
/// `sh` as before.
fn from_shebang(first_line: &str) -> Language {
    const INTERPRETERS: [(&str, Language); 9] = [
        ("python", Language::Py),
        ("node", Language::Js),
        ("ruby", Language::Rb),
        ("perl", Language::Perl),
        ("lua", Language::Lua),
        ("php", Language::Php),
        ("pwsh", Language::Ps1),
        ("Rscript", Language::R),
        ("elixir", Language::Ex),
    ];

    let mut words = first_line.trim_start_matches("#!").split_whitespace();
    let program = words.next().unwrap_or_default();
    let program = if program.ends_with("/env") {
        words
            .find(|word| !word.starts_with('-'))
            .unwrap_or_default()
    } else {
        program
    };
    let name = program.rsplit('/').next().unwrap_or_default();

    INTERPRETERS
        .iter()
        .find(|(interpreter, _)| name.starts_with(interpreter))
        .map_or(Language::Sh, |(_, language)| *language)
}

/// The languages below are guessed on a signature or not at all: what else they write only adds
/// to it, so none of them can tie an older language on a marker they share.
fn signed(signature: bool, extra: u32) -> u32 {
    if signature { SIGNATURE + extra } else { 0 }
}

/// What C never writes: the standard library's namespace, a stream, a template, a header
/// with no `.h` — which earns C's `#include` too, so it is worth more here.
fn score_cpp(sample: &Sample<'_>) -> u32 {
    const MARKERS: [&str; 10] = [
        "std::cout",
        "std::cerr",
        "std::endl",
        "std::vector<",
        "std::unique_ptr<",
        "std::shared_ptr<",
        "std::make_unique<",
        "using namespace std;",
        "template <typename",
        "template<typename",
    ];
    const HEADERS: [&str; 8] = [
        "#include <iostream>",
        "#include <vector>",
        "#include <string>",
        "#include <memory>",
        "#include <map>",
        "#include <algorithm>",
        "#include <thread>",
        "#include <array>",
    ];

    let header = sample.line_starts_with_any(&HEADERS);
    signed(
        header || MARKERS.iter().any(|marker| sample.has(marker)),
        worth(STRONG, header)
            + worth(
                STRONG,
                sample.line_starts_with_any(&["public:", "private:", "protected:"]),
            ),
    )
}

/// Markup in a script, with what React alone writes in it.
fn jsx_markup(sample: &Sample<'_>) -> bool {
    const ATTRIBUTES: [&str; 6] = [
        "className=",
        "onClick={",
        "onChange={",
        "onSubmit={",
        "useState(",
        "useEffect(",
    ];

    (sample.has("/>") || sample.has("</")) && ATTRIBUTES.iter().any(|marker| sample.has(marker))
}

fn script_keyword(sample: &Sample<'_>) -> bool {
    sample.line_starts_with_any(&["import ", "export ", "const ", "function ", "return "])
}

/// The markup is worth what HTML's tags are, on top: a component outscores the page.
fn score_jsx(sample: &Sample<'_>) -> u32 {
    signed(
        jsx_markup(sample),
        STRONG + worth(STRONG, script_keyword(sample)),
    )
}

/// JSX plus TypeScript's own score, so a typed component outscores both, and nothing without
/// a type.
fn score_tsx(sample: &Sample<'_>) -> u32 {
    let typed = score_typescript(sample);
    signed(
        jsx_markup(sample) && typed > 0,
        STRONG + typed + worth(STRONG, script_keyword(sample)),
    )
}

fn score_vue(sample: &Sample<'_>) -> u32 {
    const DIRECTIVES: [&str; 7] = [
        "v-if=",
        "v-for=",
        "v-model=",
        "v-bind:",
        "v-on:",
        "v-else",
        "<script setup",
    ];

    let component = sample.line_starts_with_any(&["<template"])
        && sample.has("</template>")
        && sample.has("<script");
    signed(
        component || DIRECTIVES.iter().any(|marker| sample.has(marker)),
        worth(STRONG, sample.has("</")),
    )
}

fn score_angular(sample: &Sample<'_>) -> u32 {
    const MARKERS: [&str; 8] = [
        "*ngIf=",
        "*ngFor=",
        "[(ngModel)]",
        "[formGroup]",
        "formControlName=",
        "routerLink",
        "; track ",
        "| async",
    ];

    signed(
        MARKERS.iter().any(|marker| sample.has(marker)),
        worth(STRONG, sample.has("</")),
    )
}

/// `@name: value;` — an at-rule never has a colon right after a bare name.
fn score_less(sample: &Sample<'_>) -> u32 {
    let variable = sample.any_line(|line| {
        line.strip_prefix('@').is_some_and(|rest| {
            rest.split_once(':')
                .is_some_and(|(name, _)| !name.is_empty() && name.chars().all(is_identifier_char))
        })
    });

    signed(variable, 0)
}

fn score_dockerfile(sample: &Sample<'_>) -> u32 {
    const INSTRUCTIONS: [&str; 9] = [
        "RUN ",
        "COPY ",
        "ADD ",
        "WORKDIR ",
        "ENTRYPOINT ",
        "CMD ",
        "EXPOSE ",
        "ENV ",
        "ARG ",
    ];

    signed(
        sample.line_starts_with_any(&["FROM "]) && sample.line_starts_with_any(&INSTRUCTIONS),
        0,
    )
}

fn score_hcl(sample: &Sample<'_>) -> u32 {
    const BLOCKS: [&str; 7] = [
        "resource \"",
        "provider \"",
        "variable \"",
        "module \"",
        "output \"",
        "data \"",
        "terraform {",
    ];

    signed(
        sample.any_line(|line| starts_with_any(line, &BLOCKS) && line.ends_with('{')),
        worth(STRONG, sample.has("${var.") || sample.has(" = var.")),
    )
}

fn score_nginx(sample: &Sample<'_>) -> u32 {
    signed(
        sample.line_starts_with_any(&["proxy_pass ", "server_name ", "try_files "]),
        worth(
            STRONG,
            sample.any_line(|line| line.starts_with("location ") && line.ends_with('{')),
        ),
    )
}

fn score_proto(sample: &Sample<'_>) -> u32 {
    signed(
        sample.has("syntax = \"proto3\"") || sample.has("syntax = \"proto2\""),
        worth(STRONG, sample.line_starts_with_any(&["message "])),
    )
}

/// `local x = …` with its spaces: a shell's `local x=…` has none. A line ending on `then`
/// with no `;` is not a shell's `if [ … ]; then`.
fn score_lua(sample: &Sample<'_>) -> u32 {
    signed(
        sample.any_line(|line| {
            line.starts_with("local function ")
                || (line.starts_with("local ") && line.contains(" = "))
        }),
        worth(
            STRONG,
            sample.any_line(|line| line.ends_with(" then") && !line.contains(';')),
        ),
    )
}

/// A type signature on a line of its own, which Elixir's `@spec` never opens.
fn score_haskell(sample: &Sample<'_>) -> u32 {
    let signature = sample.any_line(|line| {
        line.split_once(" :: ").is_some_and(|(name, _)| {
            name.chars().next().is_some_and(|c| c.is_ascii_lowercase())
                && name
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '\''))
        })
    });
    let module = sample.any_line(|line| line.starts_with("module ") && line.ends_with(" where"));

    signed(
        signature || module || sample.line_starts_with_any(&["import qualified "]),
        worth(STRONG, sample.has(" = do")),
    )
}

fn score_ocaml(sample: &Sample<'_>) -> u32 {
    signed(
        sample.has("let () =")
            || sample.has("Printf.printf")
            || sample.has("print_endline")
            || sample.has("= struct")
            || sample.any_line(|line| line.ends_with(";;")),
        worth(
            STRONG,
            sample.has("let rec ")
                || sample.any_line(|line| line.starts_with("| ") && line.contains("->")),
        ),
    )
}

fn score_clojure(sample: &Sample<'_>) -> u32 {
    signed(
        sample.line_starts_with_any(&["(defn ", "(defn- ", "(ns ", "(defmacro "]),
        worth(STRONG, sample.has("(let [")),
    )
}

/// Clojure writes `(defmacro` too, and its own `(defn` beside it.
fn score_lisp(sample: &Sample<'_>) -> u32 {
    signed(
        sample.line_starts_with_any(&["(defun ", "(defvar ", "(defparameter ", "(defmacro "])
            && !sample.has("(defn"),
        worth(STRONG, sample.has("(setf ") || sample.has("(format t ")),
    )
}

fn score_scheme(sample: &Sample<'_>) -> u32 {
    signed(
        sample.line_starts_with_any(&[
            "(define (",
            "(define-syntax ",
            "(define-record-type ",
            "#lang ",
        ]),
        worth(STRONG, sample.has("(lambda (")),
    )
}

fn score_r(sample: &Sample<'_>) -> u32 {
    signed(
        sample.has("<- function(")
            || sample.has("data.frame(")
            || sample.line_starts_with_any(&["library("]),
        worth(STRONG, sample.has(" <- ")),
    )
}

fn score_cmake(sample: &Sample<'_>) -> u32 {
    const COMMANDS: [&str; 5] = [
        "cmake_minimum_required(",
        "add_executable(",
        "add_library(",
        "target_link_libraries(",
        "find_package(",
    ];

    signed(
        sample.line_starts_with_any(&COMMANDS),
        worth(STRONG, sample.line_starts_with_any(&["project(", "set("])),
    )
}

fn score_erlang(sample: &Sample<'_>) -> u32 {
    signed(
        sample.line_starts_with_any(&["-module(", "-export(["]) || sample.has("io:format("),
        0,
    )
}

fn score_elm(sample: &Sample<'_>) -> u32 {
    signed(
        sample.any_line(|line| line.starts_with("module ") && line.contains(" exposing ("))
            || sample.line_starts_with_any(&["type alias "]),
        worth(
            STRONG,
            sample.any_line(|line| line.starts_with("import ") && line.contains(" exposing ")),
        ),
    )
}

/// `my` before a sigil: PHP writes the `$`, never the `my`.
fn score_perl(sample: &Sample<'_>) -> u32 {
    signed(
        sample.line_starts_with_any(&["use strict;", "use warnings;", "my $", "my @", "my %"]),
        worth(STRONG, sample.has(" =~ ")),
    )
}

/// Gradle's Groovy, whose dependencies are strings with no parentheses: Kotlin's are calls. A
/// `println` with none opens the line, never a Clojure form's.
fn score_groovy(sample: &Sample<'_>) -> u32 {
    signed(
        sample.has("apply plugin:")
            || sample.line_starts_with_any(&["println \""])
            || sample.line_starts_with_any(&["implementation '", "testImplementation '", "api '"]),
        worth(
            STRONG,
            sample.any_line(|line| line.starts_with("def ") && line.contains(" = ")),
        ),
    )
}

fn score_verilog(sample: &Sample<'_>) -> u32 {
    signed(
        sample.line_starts_with_any(&["endmodule"])
            || sample.has("always @(")
            || sample.has("posedge "),
        worth(
            STRONG,
            sample.line_starts_with_any(&["reg ", "wire ", "assign "]),
        ),
    )
}

fn score_vhdl(sample: &Sample<'_>) -> u32 {
    signed(
        sample.lower.contains("library ieee;")
            || sample.lower.contains("std_logic")
            || sample.any_line(|line| {
                let line = line.to_lowercase();
                (line.starts_with("entity ") && line.ends_with(" is"))
                    || (line.starts_with("architecture ") && line.contains(" of "))
            }),
        0,
    )
}

/// Its structure outweighs the language of the lines it changes, which score in it as well.
fn score_diff(sample: &Sample<'_>) -> u32 {
    let hunk = sample.any_line(|line| {
        line.strip_prefix("@@ -")
            .is_some_and(|rest| rest.contains(" @@"))
    });
    let files = sample.line_starts_with_any(&["--- "]) && sample.line_starts_with_any(&["+++ "]);

    let git = sample.line_starts_with_any(&["diff --git "]);
    signed(
        hunk || files || git,
        worth(SIGNATURE, hunk) + worth(STRONG, files) + worth(STRONG, git),
    )
}

/// A request line or a status line, first: a body may hold anything.
fn score_http(sample: &Sample<'_>) -> u32 {
    const METHODS: [&str; 7] = [
        "GET ", "POST ", "PUT ", "PATCH ", "DELETE ", "HEAD ", "OPTIONS ",
    ];

    let first = sample.lines.first().copied().unwrap_or_default();
    let request = starts_with_any(first, &METHODS) && first.contains(" HTTP/");
    let status = first.starts_with("HTTP/1.") || first.starts_with("HTTP/2 ");

    signed(request || status, 0)
}

fn score_tex(sample: &Sample<'_>) -> u32 {
    signed(
        sample.has("\\documentclass")
            || sample.has("\\usepackage")
            || sample.has("\\begin{document}"),
        worth(
            STRONG,
            (sample.has("\\begin{") && sample.has("\\end{")) || sample.has("\\section{"),
        ),
    )
}

/// Its keywords are a YAML mapping too, which only one of them would make it.
fn score_gherkin(sample: &Sample<'_>) -> u32 {
    signed(
        sample.line_starts_with_any(&["Feature:"])
            && sample.line_starts_with_any(&["Scenario:", "Scenario Outline:", "Background:"]),
        worth(
            STRONG,
            sample.line_starts_with_any(&["Given "]) && sample.line_starts_with_any(&["Then "]),
        ),
    )
}

fn score_scala(sample: &Sample<'_>) -> u32 {
    signed(
        sample.has(" extends App")
            || sample.has("def main(args: Array[String])")
            || sample.line_starts_with_any(&["case class ", "import scala.", "sealed trait "]),
        0,
    )
}

fn score_objc(sample: &Sample<'_>) -> u32 {
    signed(
        sample.line_starts_with_any(&["#import ", "@interface ", "@implementation "])
            || sample.has("NSLog(@\""),
        worth(
            STRONG,
            sample.has("NSString *") || sample.any_line(|line| line == "@end"),
        ),
    )
}

fn score_glsl(sample: &Sample<'_>) -> u32 {
    signed(
        sample.has("gl_Position")
            || sample.has("gl_FragColor")
            || sample.has("gl_FragCoord")
            || (sample.line_starts_with_any(&["#version "]) && sample.has("void main")),
        worth(
            STRONG,
            sample.line_starts_with_any(&["uniform ", "varying ", "in vec", "out vec"]),
        ),
    )
}

fn score_cobol(sample: &Sample<'_>) -> u32 {
    signed(
        sample.lower.contains("identification division")
            || sample.lower.contains("procedure division")
            || sample.lower.contains("working-storage section"),
        0,
    )
}

fn score_fortran(sample: &Sample<'_>) -> u32 {
    signed(
        sample.lower.contains("implicit none")
            || sample.lower.contains("end program")
            || sample.lower.contains("end subroutine"),
        worth(
            STRONG,
            sample.lower.contains("integer ::") || sample.lower.contains("real ::"),
        ),
    )
}

fn score_sparql(sample: &Sample<'_>) -> u32 {
    signed(
        sample.lower.contains("select ?")
            || sample.any_line(|line| {
                line.to_lowercase().starts_with("prefix ") && line.contains(": <")
            }),
        0,
    )
}

/// A node in parentheses after `MATCH`, or a relationship drawn as an arrow.
fn score_cypher(sample: &Sample<'_>) -> u32 {
    signed(
        sample.has("-[:")
            || sample.any_line(|line| {
                let line = line.to_lowercase();
                line.starts_with("match (") || line.starts_with("merge (")
            }),
        worth(STRONG, sample.lower.contains("return ")),
    )
}

fn score_zig(sample: &Sample<'_>) -> u32 {
    signed(
        sample.has("@import(\"") || sample.has("std.debug.print("),
        worth(STRONG, sample.has("!void") || sample.has("comptime ")),
    )
}

fn score_nix(sample: &Sample<'_>) -> u32 {
    signed(
        sample.has("<nixpkgs>")
            || sample.has("mkDerivation")
            || sample.has("with pkgs;")
            || sample.has("{ pkgs, ")
            || sample.has("{ pkgs ? "),
        worth(STRONG, sample.line_starts_with_any(&["inherit "])),
    )
}

/// A model is TypeScript's or GraphQL's shape until an attribute says otherwise.
fn score_prisma(sample: &Sample<'_>) -> u32 {
    const ATTRIBUTES: [&str; 4] = ["@id", "@default(", "@relation(", "@unique"];

    let block = sample.any_line(|line| {
        (line.starts_with("datasource ") || line.starts_with("generator ")) && line.ends_with('{')
    });
    let model = sample.any_line(|line| line.starts_with("model ") && line.ends_with('{'))
        && ATTRIBUTES.iter().any(|attribute| sample.has(attribute));

    signed(block || model, 0)
}

fn score_solidity(sample: &Sample<'_>) -> u32 {
    signed(
        sample.has("pragma solidity")
            || sample.has("msg.sender")
            || sample.any_line(|line| line.starts_with("contract ") && line.ends_with('{')),
        0,
    )
}

/// YAML reads its dashes as a document's start: the end line says otherwise.
fn score_armor(sample: &Sample<'_>) -> u32 {
    signed(
        sample.trimmed.starts_with("-----BEGIN PGP"),
        worth(STRONG, sample.has("-----END PGP")),
    )
}

/// The tags both template languages write: a hint, never an answer on their own.
fn template_tags(sample: &Sample<'_>) -> bool {
    ["{% if ", "{% for ", "{% endif", "{% endfor"]
        .iter()
        .any(|tag| sample.has(tag))
}

fn score_jinja(sample: &Sample<'_>) -> u32 {
    const TAGS: [&str; 5] = [
        "{% extends ",
        "{% block ",
        "{% endblock",
        "{% macro ",
        "{% set ",
    ];

    signed(
        TAGS.iter().any(|tag| sample.has(tag)),
        worth(STRONG, template_tags(sample)) + worth(STRONG, sample.has("</")),
    )
}

fn score_liquid(sample: &Sample<'_>) -> u32 {
    const TAGS: [&str; 6] = [
        "{% assign ",
        "{% capture ",
        "{% endcapture",
        "{% unless ",
        "{% endunless",
        "{% render ",
    ];

    signed(
        TAGS.iter().any(|tag| sample.has(tag)),
        worth(STRONG, template_tags(sample)) + worth(STRONG, sample.has("</")),
    )
}

fn score_wat(sample: &Sample<'_>) -> u32 {
    signed(
        sample.trimmed.starts_with("(module")
            || sample.has("local.get ")
            || sample.has("i32.const "),
        0,
    )
}

/// JSON's own shape plus a context: always ahead of plain JSON when it has one.
fn score_jsonld(sample: &Sample<'_>) -> u32 {
    signed(sample.has("\"@context\""), score_json(sample))
}

fn score_matlab(sample: &Sample<'_>) -> u32 {
    signed(
        sample.any_line(|line| line.starts_with("function [") && line.contains("] = ")),
        worth(STRONG, sample.has("disp(") || sample.has("zeros(")),
    )
}

fn score_d(sample: &Sample<'_>) -> u32 {
    signed(
        sample.line_starts_with_any(&["import std."]),
        worth(STRONG, sample.has("writeln(")),
    )
}

/// .NET's `Console.Write` is C#'s signature too: a block's end and a declaration outweigh it.
fn score_vb(sample: &Sample<'_>) -> u32 {
    let block =
        sample.line_starts_with_any(&["End Sub", "End Function", "End Module", "Imports System"]);
    let declaration = sample.any_line(|line| line.starts_with("Dim ") && line.contains(" As "));

    signed(
        block || declaration,
        worth(STRONG, block) + worth(STRONG, declaration),
    )
}

fn score_tcl(sample: &Sample<'_>) -> u32 {
    signed(
        sample.any_line(|line| line.starts_with("proc ") && line.ends_with('{'))
            || sample.has("[expr "),
        worth(STRONG, sample.has("puts ")),
    )
}

/// Python with C declarations: it adds to Python's score, which it then always outscores.
fn score_cython(sample: &Sample<'_>) -> u32 {
    signed(
        sample.line_starts_with_any(&["cdef ", "cpdef ", "cimport ", "from libc"]),
        score_python(sample),
    )
}

fn score_puppet(sample: &Sample<'_>) -> u32 {
    signed(sample.has("ensure => ") || sample.has("ensure  => "), 0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::notes::kind::NoteKind;
    use crate::notes::model::NoteLifecycle;
    use crate::notes::priority::Priority;

    #[test]
    fn every_variant_round_trips_through_its_stored_form() {
        for language in Language::ALL {
            assert_eq!(language.as_str().parse(), Ok(language));
        }
    }

    #[test]
    fn an_unknown_value_is_refused_rather_than_guessed() {
        assert_eq!("from-the-future".parse::<Language>(), Err(()));
        assert_eq!("JSON".parse::<Language>(), Err(()));
    }

    #[test]
    fn the_default_is_the_one_detection_replaces() {
        assert_eq!(Language::default(), Language::Txt);
    }

    #[test]
    fn the_serialized_form_matches_the_stored_one() {
        for language in Language::ALL {
            let json = serde_json::to_value(language).unwrap();
            assert_eq!(json, serde_json::json!(language.as_str()));
        }
    }

    fn draft(language: Language, content: &str) -> NoteDraft {
        NoteDraft {
            space_id: "s-1".to_string(),
            folder_id: None,
            title: String::new(),
            language,
            content: content.to_string(),
            source: String::new(),
            tags: Vec::new(),
            pinned: false,
            lifecycle: NoteLifecycle::Permanent,
            kind: NoteKind::Snippet,
            priority: Priority::None,
            items: Vec::new(),
        }
    }

    #[test]
    fn a_draft_with_no_chosen_language_gets_the_detected_one() {
        assert_eq!(for_draft(&draft(Language::Txt, "SELECT 1")), Language::Sql);
    }

    #[test]
    fn a_chosen_language_is_never_overwritten() {
        assert_eq!(for_draft(&draft(Language::Md, "SELECT 1")), Language::Md);
    }

    #[test]
    fn an_empty_draft_stays_plain_text() {
        assert_eq!(for_draft(&draft(Language::Txt, "")), Language::Txt);
    }

    fn blank_snippet() -> Note {
        Note {
            language: Language::Txt,
            content: String::new(),
            ..crate::notes::fixtures::note()
        }
    }

    fn content_patch(content: &str) -> NotePatch {
        NotePatch {
            content: Some(content.to_string()),
            ..NotePatch::default()
        }
    }

    #[test]
    fn an_empty_snippet_receiving_its_first_content_gets_a_language() {
        let detected = after_patch(
            &blank_snippet(),
            &content_patch("interface A { id: string }"),
        );

        assert_eq!(detected, Some(Language::Ts));
    }

    #[test]
    fn a_snippet_that_already_had_content_keeps_its_language() {
        let note = Note {
            content: "some prose".to_string(),
            ..blank_snippet()
        };

        assert!(after_patch(&note, &content_patch("SELECT 1")).is_none());
    }

    #[test]
    fn a_chosen_language_is_never_corrected_by_a_later_paste() {
        let note = Note {
            language: Language::Md,
            ..blank_snippet()
        };

        assert!(after_patch(&note, &content_patch("SELECT 1")).is_none());
    }

    #[test]
    fn a_patch_setting_the_language_itself_is_left_alone() {
        let patch = NotePatch {
            language: Some(Language::Md),
            ..content_patch("SELECT 1")
        };

        assert!(after_patch(&blank_snippet(), &patch).is_none());
    }

    #[test]
    fn a_patch_carrying_no_content_detects_nothing() {
        let patch = NotePatch {
            title: Some("Title".to_string()),
            ..NotePatch::default()
        };

        assert!(after_patch(&blank_snippet(), &patch).is_none());
    }

    #[test]
    fn clearing_the_content_does_not_detect() {
        assert!(after_patch(&blank_snippet(), &content_patch("   ")).is_none());
    }

    #[test]
    fn every_detected_language_is_one_the_editor_accepts() {
        let samples = [
            "",
            "#!/bin/bash\necho hi",
            "{\"a\": 1}",
            "<?xml version=\"1.0\"?><a/>",
            "<html><body>hi</body></html>",
            "SELECT 1",
            "[package]\nname = \"x\"",
            "def f():\n    pass",
            "interface A { }",
            "const a = 1",
            ".a { color: red; }",
            "$a: 1px;\n.b { margin: $a; }",
            "query { a }",
            "key: value",
            "# Title\n\n- item",
            "git status",
            "just some prose",
        ];

        for sample in samples {
            assert!(
                Language::ALL.contains(&from_content(sample)),
                "sample: {sample}"
            );
        }
    }

    #[test]
    fn an_empty_or_blank_content_stays_plain_text() {
        assert_eq!(from_content(""), Language::Txt);
        assert_eq!(from_content("   \n  "), Language::Txt);
    }

    #[test]
    fn prose_stays_plain_text() {
        assert_eq!(
            from_content("Penser à relancer Marc au sujet du certificat"),
            Language::Txt
        );
    }

    #[test]
    fn a_json_object_or_array_is_recognized() {
        assert_eq!(from_content("{\n  \"id\": 42\n}"), Language::Json);
        assert_eq!(from_content("[1, 2, 3]"), Language::Json);
        assert_eq!(from_content("  {\"a\": [1]}  "), Language::Json);
    }

    #[test]
    fn a_shebang_wins_over_everything_that_follows() {
        assert_eq!(
            from_content("#!/usr/bin/env python3\nconsole.log(1)"),
            Language::Py
        );
        assert_eq!(from_content("#!/usr/bin/perl -w\nprint 1;"), Language::Perl);
        assert_eq!(
            from_content("#!/usr/bin/env -S node --no-warnings\n1"),
            Language::Js
        );
        assert_eq!(from_content("#!/bin/zsh\nprint -l *"), Language::Sh);
        assert_eq!(from_content("#!/usr/bin/env tclsh\nputs 1"), Language::Sh);
    }

    #[test]
    fn markup_splits_between_html_and_xml() {
        assert_eq!(
            from_content("<!DOCTYPE html>\n<html></html>"),
            Language::Html
        );
        assert_eq!(from_content("<div class=\"x\">hi</div>"), Language::Html);
        assert_eq!(
            from_content("<?xml version=\"1.0\"?>\n<root/>"),
            Language::Xml
        );
        assert_eq!(from_content("<config><item/></config>"), Language::Xml);
    }

    #[test]
    fn sql_is_recognized_whatever_its_case() {
        assert_eq!(from_content("SELECT * FROM notes"), Language::Sql);
        assert_eq!(from_content("select 1"), Language::Sql);
        assert_eq!(
            from_content("CREATE TABLE notes (id TEXT PRIMARY KEY)"),
            Language::Sql
        );
    }

    #[test]
    fn toml_needs_both_a_section_and_an_assignment() {
        assert_eq!(
            from_content("[package]\nname = \"devnotes\""),
            Language::Toml
        );
        assert_ne!(from_content("[1, 2]\nx = 3"), Language::Toml);
    }

    #[test]
    fn python_is_told_apart_from_typescript_by_its_colon() {
        assert_eq!(from_content("def run():\n    return 1"), Language::Py);
        assert_eq!(from_content("class Note:\n    pass"), Language::Py);
        assert_eq!(from_content("from os import path"), Language::Py);
    }

    /// `class Note { }` is written the same way in Java, C#, PHP, Dart, TypeScript and
    /// JavaScript: one weak marker is not an answer.
    #[test]
    fn one_weak_marker_is_not_an_answer() {
        assert_eq!(from_content("class Note { }"), Language::Txt);
        assert_eq!(from_content("x => x + 1"), Language::Txt);
    }

    #[test]
    fn a_compiled_language_is_not_taken_for_javascript() {
        assert_eq!(
            from_content(
                "fn main() {
    println!(\"hi\");
}"
            ),
            Language::Rs
        );
        assert_eq!(
            from_content(
                "match value {
    Some(x) => x,
}
let mut total = 0;"
            ),
            Language::Rs
        );
        assert_eq!(
            from_content(
                "package main

func main() {
    fmt.Println(\"hi\")
}"
            ),
            Language::Go
        );
        assert_eq!(
            from_content(
                "import java.util.List;

class Note { }"
            ),
            Language::Java
        );
        assert_eq!(
            from_content(
                "using System;

var greet = () => Console.WriteLine(\"hi\");"
            ),
            Language::Cs
        );
        assert_eq!(
            from_content(
                "#include <stdio.h>

int main(void) { return 0; }"
            ),
            Language::C
        );
    }

    /// `<?php` opens with a `<`, which the markup check reads as a processing instruction.
    #[test]
    fn php_is_recognised_before_the_markup_check_claims_it() {
        assert_eq!(
            from_content(
                "<?php

echo 'hi';"
            ),
            Language::Php
        );
        assert_eq!(from_content("<?xml version=\"1.0\"?>"), Language::Xml);
    }

    #[test]
    fn the_languages_detected_before_them_are_left_alone() {
        assert_eq!(from_content("export enum Kind { A }"), Language::Ts);
        assert_eq!(from_content("interface Note { id: string }"), Language::Ts);
        assert_eq!(
            from_content(
                "def run():
    return 1"
            ),
            Language::Py
        );
        assert_eq!(from_content("select 1"), Language::Sql);
        assert_eq!(from_content("const add = (a, b) => a + b"), Language::Js);
    }

    #[test]
    fn typescript_wins_over_javascript_on_its_own_markers() {
        assert_eq!(from_content("interface Note { id: string }"), Language::Ts);
        assert_eq!(from_content("export type Id = string"), Language::Ts);
        assert_eq!(from_content("const a: number = 1"), Language::Ts);
        assert_eq!(from_content("const add = (a, b) => a + b"), Language::Js);
        assert_eq!(from_content("console.log('hi')"), Language::Js);
    }

    #[test]
    fn css_needs_a_selector_and_a_declaration() {
        assert_eq!(from_content(".card {\n  color: red;\n}"), Language::Css);
        assert_eq!(
            from_content("@media print {\n  a { color: #000; }\n}"),
            Language::Css
        );
    }

    #[test]
    fn scss_is_css_with_a_variable_a_mixin_or_nesting() {
        assert_eq!(
            from_content("$gap: 8px;\n.card {\n  padding: $gap;\n}"),
            Language::Scss
        );
        assert_eq!(
            from_content("@use 'mixins' as *;\n.card {\n  @include surface;\n}"),
            Language::Scss
        );
        assert_eq!(
            from_content(".card {\n  color: red;\n  &:hover {\n    color: blue;\n  }\n}"),
            Language::Scss
        );
        assert_eq!(from_content(".card {\n  color: red;\n}"), Language::Css);
    }

    #[test]
    fn graphql_is_told_apart_from_typescript_by_its_non_null_types() {
        assert_eq!(
            from_content("query GetUser($id: ID!) {\n  user(id: $id) {\n    name\n  }\n}"),
            Language::Graphql
        );
        assert_eq!(
            from_content("fragment Card on Note {\n  id\n  title\n}"),
            Language::Graphql
        );
        assert_eq!(
            from_content("type Note {\n  id: ID!\n  tags: [String!]!\n}"),
            Language::Graphql
        );
        assert_eq!(
            from_content("type Note = {\n  id: string;\n}"),
            Language::Ts
        );
        assert_ne!(from_content("the query ran twice"), Language::Graphql);
    }

    #[test]
    fn yaml_is_recognized_by_its_mappings_and_lists() {
        assert_eq!(from_content("name: devnotes\nversion: 1"), Language::Yml);
        assert_eq!(from_content("---\nsteps:\n  - build"), Language::Yml);
    }

    #[test]
    fn a_bare_url_is_not_read_as_a_yaml_mapping() {
        assert_ne!(from_content("https://example.com/a/b"), Language::Yml);
    }

    #[test]
    fn markdown_is_recognized_by_its_headings_and_fences() {
        assert_eq!(from_content("# Heading\n\nA paragraph."), Language::Md);
        assert_eq!(from_content("Voir ```code``` ici"), Language::Md);
        assert_eq!(from_content("Un [lien](https://x.dev)"), Language::Md);
    }

    #[test]
    fn shell_commands_are_the_last_resort() {
        assert_eq!(from_content("git status\ngit push"), Language::Sh);
        assert_eq!(from_content("docker run -p 8080:80 nginx"), Language::Sh);
    }
}
