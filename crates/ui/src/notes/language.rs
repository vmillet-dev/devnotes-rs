//! What the interface calls each language: the badge's short label, the name in full, and the
//! hue of its family. Names of languages, the same in every locale, as in the reference.

use engine::notes::language::Language;

/// Six hues by family: the label names the language, the colour only groups it at a glance.
/// Blue is typed and read before it runs, amber what a page runs, green what a machine runs,
/// purple data and what shapes it, red configuration, neutral prose.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hue {
    Blue,
    Amber,
    Green,
    Purple,
    Red,
    Neutral,
}

/// The badge's, and the menu's: « TS ».
pub fn label(language: Language) -> &'static str {
    info(language).0
}

/// What it is called in a sentence: « TypeScript ».
pub fn name(language: Language) -> &'static str {
    info(language).1
}

pub fn hue(language: Language) -> Hue {
    info(language).2
}

#[expect(clippy::too_many_lines, reason = "one arm a language")]
fn info(language: Language) -> (&'static str, &'static str, Hue) {
    match language {
        Language::Json => ("JSON", "JSON", Hue::Blue),
        Language::Js => ("JS", "JavaScript", Hue::Amber),
        Language::Jsx => ("JSX", "JSX", Hue::Amber),
        Language::Ts => ("TS", "TypeScript", Hue::Blue),
        Language::Tsx => ("TSX", "TSX", Hue::Blue),
        Language::Py => ("PY", "Python", Hue::Green),
        Language::Rs => ("RS", "Rust", Hue::Green),
        Language::Go => ("GO", "Go", Hue::Green),
        Language::Java => ("JAVA", "Java", Hue::Blue),
        Language::Cs => ("CS", "C#", Hue::Blue),
        Language::Php => ("PHP", "PHP", Hue::Amber),
        Language::C => ("C", "C", Hue::Green),
        Language::Cpp => ("C++", "C++", Hue::Green),
        Language::Sql => ("SQL", "SQL", Hue::Purple),
        Language::Graphql => ("GRAPHQL", "GraphQL", Hue::Purple),
        Language::Yml => ("YML", "YAML", Hue::Red),
        Language::Toml => ("TOML", "TOML", Hue::Red),
        Language::Xml => ("XML", "XML", Hue::Blue),
        Language::Html => ("HTML", "HTML", Hue::Amber),
        Language::Vue => ("VUE", "Vue", Hue::Amber),
        Language::Angular => ("NG", "Angular template", Hue::Amber),
        Language::Css => ("CSS", "CSS", Hue::Purple),
        Language::Scss => ("SCSS", "SCSS", Hue::Purple),
        Language::Less => ("LESS", "Less", Hue::Purple),
        Language::Sass => ("SASS", "Sass", Hue::Purple),
        Language::Sh => ("SH", "Shell", Hue::Green),
        Language::Ps1 => ("PS1", "PowerShell", Hue::Green),
        Language::Kt => ("KT", "Kotlin", Hue::Green),
        Language::Rb => ("RB", "Ruby", Hue::Green),
        Language::Swift => ("SWIFT", "Swift", Hue::Green),
        Language::Dart => ("DART", "Dart", Hue::Green),
        Language::Ex => ("EX", "Elixir", Hue::Green),
        Language::Apl => ("APL", "APL", Hue::Green),
        Language::Armasm => ("ARM", "ARM assembly", Hue::Green),
        Language::Asn1 => ("ASN.1", "ASN.1", Hue::Blue),
        Language::Asm => ("ASM", "Assembly (GAS)", Hue::Green),
        Language::Asterisk => ("ASTERISK", "Asterisk dialplan", Hue::Red),
        Language::Bf => ("BF", "Brainfuck", Hue::Green),
        Language::Ceylon => ("CEYLON", "Ceylon", Hue::Blue),
        Language::Clj => ("CLJ", "Clojure", Hue::Green),
        Language::Gss => ("GSS", "Closure Stylesheets", Hue::Purple),
        Language::Cmake => ("CMAKE", "CMake", Hue::Red),
        Language::Cobol => ("COBOL", "COBOL", Hue::Green),
        Language::Coffee => ("COFFEE", "CoffeeScript", Hue::Amber),
        Language::Lisp => ("LISP", "Common Lisp", Hue::Green),
        Language::Cr => ("CR", "Crystal", Hue::Green),
        Language::Cypher => ("CYPHER", "Cypher", Hue::Purple),
        Language::Cython => ("CYTHON", "Cython", Hue::Green),
        Language::D => ("D", "D", Hue::Green),
        Language::Diff => ("DIFF", "Diff", Hue::Neutral),
        Language::Dockerfile => ("DOCKER", "Dockerfile", Hue::Red),
        Language::Dtd => ("DTD", "DTD", Hue::Blue),
        Language::Dylan => ("DYLAN", "Dylan", Hue::Green),
        Language::Ebnf => ("EBNF", "EBNF", Hue::Purple),
        Language::Ecl => ("ECL", "ECL", Hue::Purple),
        Language::Eiffel => ("EIFFEL", "Eiffel", Hue::Blue),
        Language::Elm => ("ELM", "Elm", Hue::Amber),
        Language::Erl => ("ERL", "Erlang", Hue::Green),
        Language::Fs => ("F#", "F#", Hue::Blue),
        Language::Factor => ("FACTOR", "Factor", Hue::Green),
        Language::Fcl => ("FCL", "FCL", Hue::Green),
        Language::Forth => ("FORTH", "Forth", Hue::Green),
        Language::Fortran => ("FORTRAN", "Fortran", Hue::Green),
        Language::Gherkin => ("GHERKIN", "Gherkin", Hue::Neutral),
        Language::Glsl => ("GLSL", "GLSL", Hue::Green),
        Language::Groovy => ("GROOVY", "Groovy", Hue::Green),
        Language::Hs => ("HS", "Haskell", Hue::Blue),
        Language::Haxe => ("HAXE", "Haxe", Hue::Blue),
        Language::Http => ("HTTP", "HTTP", Hue::Amber),
        Language::Hxml => ("HXML", "HXML", Hue::Red),
        Language::Idl => ("IDL", "IDL", Hue::Blue),
        Language::Ini => ("INI", "INI", Hue::Red),
        Language::Jinja => ("JINJA", "Jinja", Hue::Amber),
        Language::Jsonld => ("JSON-LD", "JSON-LD", Hue::Blue),
        Language::Jl => ("JL", "Julia", Hue::Green),
        Language::Tex => ("TEX", "LaTeX", Hue::Neutral),
        Language::Liquid => ("LIQUID", "Liquid", Hue::Amber),
        Language::Ls => ("LS", "LiveScript", Hue::Amber),
        Language::Lua => ("LUA", "Lua", Hue::Green),
        Language::Mathematica => ("WL", "Mathematica", Hue::Green),
        Language::Matlab => ("MATLAB", "MATLAB / Octave", Hue::Green),
        Language::Mbox => ("MBOX", "mbox", Hue::Neutral),
        Language::Mirc => ("MIRC", "mIRC", Hue::Green),
        Language::Modelica => ("MODELICA", "Modelica", Hue::Blue),
        Language::Mscgen => ("MSC", "Mscgen", Hue::Purple),
        Language::Mumps => ("MUMPS", "MUMPS", Hue::Green),
        Language::Ntriples => ("NT", "N-Triples", Hue::Purple),
        Language::Nesc => ("NESC", "nesC", Hue::Green),
        Language::Nginx => ("NGINX", "Nginx", Hue::Red),
        Language::Nix => ("NIX", "Nix", Hue::Red),
        Language::Nsis => ("NSIS", "NSIS", Hue::Red),
        Language::Objc => ("OBJC", "Objective-C", Hue::Green),
        Language::Objcpp => ("OBJC++", "Objective-C++", Hue::Green),
        Language::Ml => ("ML", "OCaml", Hue::Blue),
        Language::Oz => ("OZ", "Oz", Hue::Green),
        Language::Pascal => ("PASCAL", "Pascal", Hue::Green),
        Language::Pegjs => ("PEGJS", "PEG.js", Hue::Purple),
        Language::Perl => ("PERL", "Perl", Hue::Green),
        Language::Asc => ("PGP", "PGP armor", Hue::Neutral),
        Language::Pig => ("PIG", "Pig", Hue::Purple),
        Language::Prisma => ("PRISMA", "Prisma", Hue::Purple),
        Language::Properties => ("PROPS", "Properties", Hue::Red),
        Language::Proto => ("PROTO", "Protocol Buffers", Hue::Blue),
        Language::Pug => ("PUG", "Pug", Hue::Amber),
        Language::Puppet => ("PUPPET", "Puppet", Hue::Red),
        Language::Q => ("Q", "Q", Hue::Purple),
        Language::R => ("R", "R", Hue::Purple),
        Language::Rpmspec => ("RPM", "RPM spec", Hue::Red),
        Language::Sas => ("SAS", "SAS", Hue::Purple),
        Language::Scala => ("SCALA", "Scala", Hue::Blue),
        Language::Scheme => ("SCHEME", "Scheme", Hue::Green),
        Language::Sieve => ("SIEVE", "Sieve", Hue::Red),
        Language::Smalltalk => ("ST", "Smalltalk", Hue::Green),
        Language::Sol => ("SOL", "Solidity", Hue::Green),
        Language::Solr => ("SOLR", "Solr", Hue::Purple),
        Language::Sparql => ("SPARQL", "SPARQL", Hue::Purple),
        Language::Spreadsheet => ("SHEET", "Spreadsheet formula", Hue::Purple),
        Language::Squirrel => ("NUT", "Squirrel", Hue::Green),
        Language::Sml => ("SML", "Standard ML", Hue::Blue),
        Language::Styl => ("STYL", "Stylus", Hue::Purple),
        Language::Tcl => ("TCL", "Tcl", Hue::Green),
        Language::Tf => ("TF", "Terraform / HCL", Hue::Red),
        Language::Textile => ("TEXTILE", "Textile", Hue::Neutral),
        Language::Tiddlywiki => ("TW", "TiddlyWiki", Hue::Neutral),
        Language::Tiki => ("TIKI", "Tiki wiki", Hue::Neutral),
        Language::Troff => ("TROFF", "troff", Hue::Neutral),
        Language::Ttcn => ("TTCN", "TTCN-3", Hue::Blue),
        Language::Ttcncfg => ("TTCN-CFG", "TTCN-3 configuration", Hue::Red),
        Language::Turtle => ("TTL", "Turtle", Hue::Purple),
        Language::Vbs => ("VBS", "VBScript", Hue::Green),
        Language::Velocity => ("VTL", "Velocity", Hue::Amber),
        Language::Verilog => ("VERILOG", "Verilog", Hue::Green),
        Language::Vhdl => ("VHDL", "VHDL", Hue::Green),
        Language::Vb => ("VB", "Visual Basic", Hue::Blue),
        Language::Webidl => ("WEBIDL", "Web IDL", Hue::Blue),
        Language::Wat => ("WAT", "WebAssembly text", Hue::Amber),
        Language::Xquery => ("XQUERY", "XQuery", Hue::Purple),
        Language::Yacas => ("YACAS", "Yacas", Hue::Green),
        Language::Z80 => ("Z80", "Z80 assembly", Hue::Green),
        Language::Zig => ("ZIG", "Zig", Hue::Green),
        Language::Md => ("MD", "Markdown", Hue::Neutral),
        Language::Txt => ("TXT", "Text", Hue::Neutral),
    }
}

/// The grammar `text::highlight` colours a language with; `None` is plain text, where no grammar
/// bundled reads it (gap E11).
#[expect(clippy::too_many_lines, reason = "one arm a language")]
pub fn syntax(language: Language) -> Option<&'static str> {
    let name = match language {
        Language::Json | Language::Jsonld => "JSON",
        Language::Js => "JavaScript",
        Language::Jsx => "JavaScript (Babel)",
        Language::Ts => "TypeScript",
        Language::Tsx => "TypeScriptReact",
        Language::Py | Language::Cython => "Python",
        Language::Rs => "Rust",
        Language::Go => "Go",
        Language::Java => "Java",
        Language::Cs => "C#",
        Language::Php => "PHP",
        Language::C | Language::Nesc => "C",
        Language::Cpp => "C++",
        Language::Sql => "SQL",
        Language::Graphql => "GraphQL",
        Language::Yml => "YAML",
        Language::Toml => "TOML",
        Language::Xml | Language::Dtd => "XML",
        Language::Html | Language::Angular => "HTML",
        Language::Vue => "Vue Component",
        Language::Css | Language::Gss => "CSS",
        Language::Scss => "SCSS",
        Language::Less => "Less",
        Language::Sass => "Sass",
        Language::Sh => "Bourne Again Shell (bash)",
        Language::Ps1 => "PowerShell",
        Language::Kt => "Kotlin",
        Language::Rb => "Ruby",
        Language::Swift => "Swift",
        Language::Dart => "Dart",
        Language::Ex => "Elixir",
        Language::Armasm => "ARM Assembly",
        Language::Asm | Language::Z80 => "x86_64 Assembly",
        Language::Clj => "Clojure",
        Language::Cmake => "CMake",
        Language::Coffee => "CoffeeScript",
        Language::Lisp | Language::Scheme => "Lisp",
        Language::Cr => "Crystal",
        Language::D => "D",
        Language::Diff => "Diff",
        Language::Dockerfile => "Dockerfile",
        Language::Elm => "Elm",
        Language::Erl => "Erlang",
        Language::Fs => "F#",
        Language::Fortran => "Fortran (Modern)",
        Language::Glsl => "GLSL",
        Language::Groovy => "Groovy",
        Language::Hs => "Haskell",
        Language::Http => "HTTP Request and Response",
        Language::Ini | Language::Asterisk => "INI",
        Language::Jinja => "Jinja2",
        Language::Jl => "Julia",
        Language::Tex => "LaTeX",
        Language::Ls => "LiveScript",
        Language::Lua => "Lua",
        Language::Matlab => "MATLAB",
        Language::Mbox => "Email",
        Language::Nginx => "nginx",
        Language::Nix => "Nix",
        Language::Nsis => "NSIS",
        Language::Objc => "Objective-C",
        Language::Objcpp => "Objective-C++",
        Language::Ml => "OCaml",
        Language::Pascal => "Pascal",
        Language::Perl => "Perl",
        Language::Prisma => "Prisma",
        Language::Properties => "Java Properties",
        Language::Proto => "Protocol Buffer",
        Language::Puppet => "Puppet",
        Language::R => "R",
        Language::Scala => "Scala",
        Language::Sol => "Solidity",
        Language::Sml => "SML",
        Language::Styl => "Stylus",
        Language::Tcl => "Tcl",
        Language::Tf => "Terraform",
        Language::Textile => "Textile",
        Language::Troff => "Groff/troff",
        Language::Verilog => "Verilog",
        Language::Vhdl => "VHDL",
        Language::Zig => "Zig",
        Language::Md => "Markdown",
        Language::Apl
        | Language::Asn1
        | Language::Bf
        | Language::Ceylon
        | Language::Cobol
        | Language::Cypher
        | Language::Dylan
        | Language::Ebnf
        | Language::Ecl
        | Language::Eiffel
        | Language::Factor
        | Language::Fcl
        | Language::Forth
        | Language::Gherkin
        | Language::Haxe
        | Language::Hxml
        | Language::Idl
        | Language::Liquid
        | Language::Mathematica
        | Language::Mirc
        | Language::Modelica
        | Language::Mscgen
        | Language::Mumps
        | Language::Ntriples
        | Language::Oz
        | Language::Pegjs
        | Language::Asc
        | Language::Pig
        | Language::Pug
        | Language::Q
        | Language::Rpmspec
        | Language::Sas
        | Language::Sieve
        | Language::Smalltalk
        | Language::Solr
        | Language::Sparql
        | Language::Spreadsheet
        | Language::Squirrel
        | Language::Tiddlywiki
        | Language::Tiki
        | Language::Ttcn
        | Language::Ttcncfg
        | Language::Turtle
        | Language::Vbs
        | Language::Velocity
        | Language::Vb
        | Language::Webidl
        | Language::Wat
        | Language::Xquery
        | Language::Yacas
        | Language::Txt => return None,
    };
    Some(name)
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn every_language_has_a_label_and_a_name_of_its_own() {
        let mut names = HashSet::new();
        for language in Language::ALL {
            assert!(!label(language).is_empty(), "{language:?}");
            assert!(names.insert(name(language)), "{language:?}");
        }
    }

    #[test]
    fn a_language_is_named_and_coloured_by_its_family() {
        assert_eq!(label(Language::Ts), "TS");
        assert_eq!(name(Language::Ts), "TypeScript");
        assert_eq!(hue(Language::Ts), Hue::Blue);
        assert_eq!(hue(Language::Js), Hue::Amber);
    }

    #[test]
    fn every_grammar_named_exists_and_the_common_languages_have_one() {
        text::highlight::warm();
        for language in Language::ALL {
            if let Some(name) = syntax(language) {
                assert!(text::highlight::has_syntax(name), "{language:?}: {name}");
            }
        }
        let common = [
            Language::Json,
            Language::Ts,
            Language::Py,
            Language::Rs,
            Language::Sql,
        ];
        assert!(
            common
                .into_iter()
                .all(|language| syntax(language).is_some())
        );
        assert_eq!(syntax(Language::Txt), None);
    }
}
