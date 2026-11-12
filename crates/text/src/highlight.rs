//! Code coloured by TextMate grammars (`syntect` with `bat`'s set from `two-face`, regular
//! expressions by `onig`, D22), the scopes read as the reference's seven `tok-*` classes. The
//! grammar is named by the caller: which one a note's language takes is `ui`'s table.
//!
//! A long text is not parsed again from its top at each edit: the parser's state is kept every
//! [`EVERY`] lines, and an edit throws away only the points after its line.

use std::collections::HashMap;
use std::ops::Range;
use std::sync::{LazyLock, OnceLock};

use syntect::parsing::{ParseState, Scope, ScopeStack, SyntaxDefinition, SyntaxSet};

/// How many lines apart the parser's state is kept.
pub const EVERY: usize = 128;

/// The reference's `tok-*` classes; what colour each is, is `ui`'s.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Token {
    Keyword,
    String,
    Literal,
    Name,
    Property,
    Meta,
    Comment,
    Deleted,
    Emphasis,
    Strong,
}

/// A line's coloured stretches, by `char` range within the line.
pub type Line = Vec<(Range<usize>, Token)>;

static SYNTAXES: OnceLock<SyntaxSet> = OnceLock::new();

fn load() -> SyntaxSet {
    let mut builder = two_face::syntax::extra_newlines().into_builder();
    let prisma = include_str!("../syntaxes/prisma.sublime-syntax");
    if let Ok(definition) = SyntaxDefinition::load_from_str(prisma, true, None) {
        builder.add(definition);
    }
    builder.build()
}

/// Loads the grammars: a few hundred milliseconds, spent on a thread at startup. Until then,
/// nothing is coloured — the interface never waits for them.
pub fn warm() {
    SYNTAXES.get_or_init(load);
}

pub fn is_ready() -> bool {
    SYNTAXES.get().is_some()
}

/// Whether a grammar of that name exists; `false` while they load.
pub fn has_syntax(name: &str) -> bool {
    SYNTAXES
        .get()
        .is_some_and(|set| set.find_syntax_by_name(name).is_some())
}

static KEY: LazyLock<Scope> =
    LazyLock::new(|| Scope::new("meta.mapping.key").expect("a valid scope"));

/// Which class the innermost scope that has one gives, a scope's answer kept.
#[derive(Default)]
struct Classes(HashMap<Scope, Option<Token>>);

impl Classes {
    fn of(&mut self, stack: &ScopeStack) -> Option<Token> {
        // A key is a property whatever it is written as, its quotes included, as Lezer reads it.
        if stack
            .as_slice()
            .iter()
            .any(|scope| KEY.is_prefix_of(*scope))
        {
            return Some(Token::Property);
        }
        stack
            .as_slice()
            .iter()
            .rev()
            .find_map(|scope| *self.0.entry(*scope).or_insert_with(|| class(*scope)))
    }
}

/// The reference's `CODE_HIGHLIGHTER`, from Lezer's tags to TextMate's scopes: the first rule a
/// scope starts with wins, the more specific first.
fn class(scope: Scope) -> Option<Token> {
    const RULES: &[(&str, Token)] = &[
        ("comment", Token::Comment),
        ("markup.deleted", Token::Deleted),
        ("markup.inserted", Token::String),
        ("markup.italic", Token::Emphasis),
        ("markup.bold", Token::Strong),
        ("markup.heading", Token::Name),
        ("markup.underline.link", Token::String),
        ("markup.quote", Token::String),
        ("string", Token::String),
        ("constant.character.escape", Token::String),
        ("constant", Token::Literal),
        ("support.constant", Token::Literal),
        ("entity.name.tag.yaml", Token::Property),
        ("entity.name.field", Token::Property),
        ("variable.function", Token::Keyword),
        ("variable.language", Token::Keyword),
        ("variable.parameter.option", Token::Property),
        ("entity.name.function", Token::Name),
        ("support.function", Token::Name),
        ("entity.name.class", Token::Name),
        ("entity.name.tag", Token::Name),
        ("entity.name.section", Token::Name),
        ("entity.name.label", Token::Name),
        ("entity.other.attribute-name", Token::Property),
        ("support.type.property-name", Token::Property),
        ("variable.other.member", Token::Property),
        ("variable.other.property", Token::Property),
        ("meta.property-name", Token::Property),
        ("entity.name.type", Token::Keyword),
        ("support.type", Token::Keyword),
        ("keyword", Token::Keyword),
        ("storage", Token::Keyword),
        ("variable", Token::Property),
        ("meta.annotation", Token::Meta),
        ("meta.decorator", Token::Meta),
        ("meta.preprocessor", Token::Meta),
        ("punctuation.definition.annotation", Token::Meta),
    ];
    let name = scope.build_string();
    RULES
        .iter()
        .find(|(prefix, _)| {
            name.starts_with(prefix) && name[prefix.len()..].chars().next().is_none_or(|c| c == '.')
        })
        .map(|(_, token)| *token)
}

/// The parser's state at the start of a line.
#[derive(Clone)]
struct Checkpoint {
    state: ParseState,
    stack: ScopeStack,
}

/// One grammar over one text, its states kept every [`EVERY`] lines.
pub struct Highlighter {
    /// `checkpoints[k]` is the state at line `k * EVERY`.
    checkpoints: Vec<Checkpoint>,
    classes: Classes,
}

impl Highlighter {
    /// `None` for a name no grammar has, or while they load: the text stays plain.
    pub fn new(name: &str) -> Option<Self> {
        let syntax = SYNTAXES.get()?.find_syntax_by_name(name)?;
        Some(Self {
            checkpoints: vec![Checkpoint {
                state: ParseState::new(syntax),
                stack: ScopeStack::new(),
            }],
            classes: Classes::default(),
        })
    }

    /// An edit from `line` on: the states after it are parsed again when asked.
    pub fn invalidate_from(&mut self, line: usize) {
        self.checkpoints.truncate(line / EVERY + 1);
    }

    /// How far the kept states reach, in lines.
    pub fn reach(&self) -> usize {
        (self.checkpoints.len() - 1) * EVERY
    }

    /// The states kept up to `line`'s block, parsing at most `budget` lines; `true` once there.
    /// `line_of(i)` gives line `i` with its line ending.
    pub fn extend(
        &mut self,
        line_of: impl Fn(usize) -> Option<String>,
        line: usize,
        budget: usize,
    ) -> bool {
        let wanted = line / EVERY;
        let mut parsed = 0;
        while self.checkpoints.len() <= wanted {
            if parsed >= budget {
                return false;
            }
            let block = self.checkpoints.len() - 1;
            let mut point = self.checkpoints[block].clone();
            for at in block * EVERY..(block + 1) * EVERY {
                let Some(text) = line_of(at) else {
                    return true;
                };
                self.step(&mut point, &text);
                parsed += 1;
            }
            self.checkpoints.push(point);
        }
        true
    }

    /// The colours of `lines`, when the kept states reach their block; `None` otherwise.
    pub fn lines(
        &mut self,
        line_of: impl Fn(usize) -> Option<String>,
        lines: Range<usize>,
    ) -> Option<Vec<Line>> {
        let block = lines.start / EVERY;
        let mut point = self.checkpoints.get(block)?.clone();
        let mut out = Vec::with_capacity(lines.len());
        for at in block * EVERY..lines.end {
            let Some(text) = line_of(at) else {
                break;
            };
            let coloured = self.step(&mut point, &text);
            if at >= lines.start {
                out.push(coloured);
            }
            if (at + 1) % EVERY == 0 && (at + 1) / EVERY == self.checkpoints.len() {
                self.checkpoints.push(point.clone());
            }
        }
        Some(out)
    }

    /// One line parsed from `point`, which moves past it.
    fn step(&mut self, point: &mut Checkpoint, text: &str) -> Line {
        let Some(set) = SYNTAXES.get() else {
            return Vec::new();
        };
        let Ok(ops) = point.state.parse_line(text, set) else {
            return Vec::new();
        };
        let mut coloured: Line = Vec::new();
        let mut from_byte = 0;
        let mut from_char = 0;
        let mut push =
            |coloured: &mut Line, stack: &ScopeStack, to_byte: usize, classes: &mut Classes| {
                let to_byte = to_byte.min(text.len());
                if to_byte <= from_byte {
                    return;
                }
                let to_char = from_char + text[from_byte..to_byte].chars().count();
                if let Some(token) = classes.of(stack) {
                    match coloured.last_mut() {
                        Some((range, last)) if *last == token && range.end == from_char => {
                            range.end = to_char;
                        }
                        _ => coloured.push((from_char..to_char, token)),
                    }
                }
                from_byte = to_byte;
                from_char = to_char;
            };
        for (at, op) in ops {
            push(&mut coloured, &point.stack, at, &mut self.classes);
            let _ = point.stack.apply(&op);
        }
        let end = text.trim_end_matches(['\n', '\r']).len();
        push(&mut coloured, &point.stack, end, &mut self.classes);
        coloured
    }
}

/// A whole text at once: what a card shows is a few lines.
pub fn highlight(text: &str, name: &str) -> Vec<Line> {
    let Some(mut highlighter) = Highlighter::new(name) else {
        return Vec::new();
    };
    let lines: Vec<String> = text.split_inclusive('\n').map(str::to_owned).collect();
    let count = lines.len();
    highlighter
        .lines(|at| lines.get(at).cloned(), 0..count)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokens(text: &str, name: &str) -> Vec<(String, Token)> {
        let lines = highlight(text, name);
        text.split_inclusive('\n')
            .zip(lines)
            .flat_map(|(line, coloured)| {
                let chars: Vec<char> = line.chars().collect();
                coloured
                    .into_iter()
                    .map(move |(range, token)| (chars[range].iter().collect::<String>(), token))
                    .collect::<Vec<_>>()
            })
            .collect()
    }

    #[test]
    fn typescript_reads_as_the_reference_s_classes() {
        warm();
        let found = tokens(
            "// note\nconst port: number = 5432;\nlog(\"up\");\n",
            "TypeScript",
        );
        assert!(
            found.contains(&("// note".into(), Token::Comment)),
            "{found:?}"
        );
        assert!(
            found.contains(&("const".into(), Token::Keyword)),
            "{found:?}"
        );
        assert!(
            found.contains(&("5432".into(), Token::Literal)),
            "{found:?}"
        );
        assert!(
            found
                .iter()
                .any(|(text, token)| text.contains("up") && *token == Token::String)
        );
        assert!(found.contains(&("log".into(), Token::Name)), "{found:?}");
    }

    #[test]
    fn a_json_key_is_a_property_and_its_value_a_string() {
        warm();
        let found = tokens("{\"host\": \"db\", \"port\": 5432}", "JSON");
        assert!(
            found
                .iter()
                .any(|(text, token)| text.contains("host") && *token == Token::Property),
            "{found:?}"
        );
        assert!(
            found.contains(&("5432".into(), Token::Literal)),
            "{found:?}"
        );
    }

    #[test]
    fn the_in_house_prisma_grammar_is_there() {
        warm();
        assert!(has_syntax("Prisma"));
        let found = tokens("model User {\n  id Int @id // key\n}\n", "Prisma");
        assert!(
            found.contains(&("model".into(), Token::Keyword)),
            "{found:?}"
        );
        assert!(found.contains(&("@id".into(), Token::Name)), "{found:?}");
        assert!(
            found.contains(&("// key".into(), Token::Comment)),
            "{found:?}"
        );
    }

    #[test]
    fn an_unknown_grammar_leaves_the_text_plain() {
        warm();
        assert!(Highlighter::new("Not a language").is_none());
        assert!(highlight("x", "Not a language").is_empty());
    }

    #[test]
    fn states_are_kept_every_block_and_an_edit_drops_only_those_after_it() {
        warm();
        let lines: Vec<String> = (0..1000).map(|n| format!("let a{n} = {n};\n")).collect();
        let line_of = |at: usize| lines.get(at).cloned();
        let mut highlighter = Highlighter::new("JavaScript").unwrap();

        assert!(
            highlighter.lines(line_of, 600..610).is_none(),
            "not reached yet"
        );
        assert!(!highlighter.extend(line_of, 600, 200));
        assert!(highlighter.extend(line_of, 600, 1000));
        assert!(highlighter.reach() + EVERY > 600);
        let coloured = highlighter.lines(line_of, 600..610).unwrap();
        assert_eq!(coloured.len(), 10);
        assert!(
            coloured[0]
                .iter()
                .any(|(_, token)| *token == Token::Keyword)
        );

        highlighter.invalidate_from(300);
        assert_eq!(highlighter.reach(), 256);
    }

    #[test]
    fn a_block_comment_carries_across_a_checkpoint() {
        warm();
        let mut lines = vec!["/*\n".to_string()];
        lines.extend((0..300).map(|_| "inside\n".to_string()));
        lines.push("*/ let x;\n".into());
        let line_of = |at: usize| lines.get(at).cloned();
        let mut highlighter = Highlighter::new("JavaScript").unwrap();
        highlighter.extend(line_of, 290, usize::MAX);

        let coloured = highlighter.lines(line_of, 290..291).unwrap();

        assert_eq!(coloured[0], vec![(0..6, Token::Comment)]);
    }

    /// The budget of § 6.2, by hand: `cargo test -p text --release budget -- --ignored
    /// --nocapture`.
    #[test]
    #[ignore = "a measure, not a check"]
    fn budget() {
        use std::fmt::Write as _;
        warm();
        let (mut ts, mut json) = (String::new(), String::new());
        for n in 0..14_000 {
            let _ = writeln!(
                ts,
                "export const f{n} = (a: number): string => `${{a}}` + \"x\"; // {n}"
            );
        }
        for n in 0..22_000 {
            let _ = writeln!(
                json,
                "  {{\"id\": {n}, \"name\": \"item {n}\", \"ok\": true}},"
            );
        }
        for (name, text) in [("TypeScript", ts), ("JSON", json)] {
            let text = &text[..text.len().min(1 << 20)];
            let lines: Vec<String> = text.split_inclusive('\n').map(str::to_owned).collect();
            let line_of = |at: usize| lines.get(at).cloned();
            let started = std::time::Instant::now();
            let mut highlighter = Highlighter::new(name).unwrap();
            highlighter.lines(line_of, 0..60);
            let screen = started.elapsed();
            highlighter.extend(line_of, lines.len(), usize::MAX);
            println!(
                "{name} 1 MB: first screen {screen:?}, whole {:?}",
                started.elapsed()
            );
        }
    }
}
