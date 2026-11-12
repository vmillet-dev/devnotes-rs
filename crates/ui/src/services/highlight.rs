//! Code coloured without ever holding the frame (D22, ADR S5). A card's few lines are coloured at
//! once and kept while drawn; a long text is coloured by windows, on a thread of its own that
//! builds the parser's states from the top: a line it has not reached is drawn plain meanwhile.

use std::collections::{HashMap, HashSet};
use std::ops::Range;
use std::sync::Arc;

use egui::Color32;
use egui::cache::{ComputerMut, FrameCache};
use text::highlight::{Highlighter, Line, Token};

use crate::theme::tokens::Palette;

/// How many lines a step parses before the thread looks for a newer window.
const STEP: usize = 2_000;

/// The reference's `_code-theme.scss`: the colour each class is drawn in, and whether it leans.
pub fn colour(p: &Palette, token: Token) -> (Color32, bool) {
    match token {
        Token::Keyword => (p.blue, false),
        Token::String => (p.green, false),
        Token::Literal => (p.amber_text, false),
        Token::Name => (p.purple, false),
        Token::Property | Token::Strong => (p.text_0, false),
        Token::Meta => (p.text_1, false),
        Token::Comment => (p.text_2, true),
        Token::Deleted => (p.red, false),
        Token::Emphasis => (p.text_1, true),
    }
}

#[derive(Default)]
struct Short;

impl ComputerMut<(&str, &str), Arc<Vec<Line>>> for Short {
    fn compute(&mut self, (syntax, code): (&str, &str)) -> Arc<Vec<Line>> {
        Arc::new(text::highlight::highlight(code, syntax))
    }
}

type ShortCache = FrameCache<Arc<Vec<Line>>, Short>;

/// A few lines — a card's — coloured, and kept while they stay on screen. Empty while the
/// grammars load: the next frame colours them.
pub fn short(ctx: &egui::Context, syntax: &str, code: &str) -> Arc<Vec<Line>> {
    if !text::highlight::is_ready() {
        return Arc::default();
    }
    ctx.memory_mut(|memory| Arc::clone(memory.caches.cache::<ShortCache>().get((syntax, code))))
}

/// The text's line `at` with its line ending, from a snapshot of the rope.
fn line_of(rope: &text::Rope) -> impl Fn(usize) -> Option<String> + '_ {
    move |at| (at < rope.len_lines()).then(|| rope.line(at).to_string())
}

/// What the frame asks the parser's thread: the window it shows, of the text as it now is.
struct Request {
    generation: u64,
    /// The first line edited since the last request; `None` for none.
    invalid_from: Option<usize>,
    rope: text::Rope,
    visible: Range<usize>,
}

struct Answer {
    generation: u64,
    start: usize,
    lines: Vec<Line>,
}

/// One long text's colours, window by window. The parser lives on a thread of its own — its
/// states are not `Send` — and answers the latest window asked; the frame draws what it has.
pub struct Colouring {
    requests: crossbeam_channel::Sender<Request>,
    answers: crossbeam_channel::Receiver<Answer>,
    lines: HashMap<usize, Arc<Line>>,
    /// The lines answered since the last edit; the others are drawn as they were.
    fresh: HashSet<usize>,
    generation: u64,
    invalid_from: Option<usize>,
    asked: Option<(u64, Range<usize>)>,
}

impl Colouring {
    /// `None` for a text no grammar reads, or while they load.
    pub fn new(ctx: &egui::Context, syntax: &str) -> Option<Self> {
        if !text::highlight::has_syntax(syntax) {
            return None;
        }
        let (requests, asked) = crossbeam_channel::unbounded::<Request>();
        let (answer, answers) = crossbeam_channel::unbounded();
        let syntax = syntax.to_owned();
        let ctx = ctx.clone();
        std::thread::Builder::new()
            .name("highlight".into())
            .spawn(move || parse(&syntax, &asked, &answer, &ctx))
            .ok()?;
        Some(Self {
            requests,
            answers,
            lines: HashMap::new(),
            fresh: HashSet::new(),
            generation: 0,
            invalid_from: None,
            asked: None,
        })
    }

    /// The text changed from `line` on. The lines after it keep their colours until the new
    /// ones arrive: plain for a frame, the line being typed would blink.
    pub fn edited(&mut self, line: usize) {
        self.generation += 1;
        self.invalid_from = Some(self.invalid_from.map_or(line, |from| from.min(line)));
        self.fresh.retain(|at| *at < line);
    }

    /// Takes what the parser answered, and asks for `visible` when it is not all known.
    pub fn window(&mut self, rope: &text::Rope, visible: Range<usize>) {
        while let Ok(answer) = self.answers.try_recv() {
            for (offset, colours) in answer.lines.into_iter().enumerate() {
                let line = answer.start + offset;
                self.lines.insert(line, Arc::new(colours));
                if answer.generation == self.generation {
                    self.fresh.insert(line);
                }
            }
        }
        let known = visible.clone().all(|line| self.fresh.contains(&line));
        let asked = Some((self.generation, visible.clone()));
        if known || self.asked == asked {
            return;
        }
        self.asked = asked;
        let request = Request {
            generation: self.generation,
            invalid_from: self.invalid_from.take(),
            rope: rope.clone(),
            visible,
        };
        // A closed thread leaves the text plain; nothing to say.
        let _ = self.requests.send(request);
    }

    /// Line `line`'s colours, when known.
    pub fn line(&self, line: usize) -> Option<&Line> {
        self.lines.get(&line).map(AsRef::as_ref)
    }
}

/// The parser's thread: the latest window asked, its states built a step at a time so a newer
/// request is seen between steps; it ends with the `Colouring`.
fn parse(
    syntax: &str,
    requests: &crossbeam_channel::Receiver<Request>,
    answers: &crossbeam_channel::Sender<Answer>,
    ctx: &egui::Context,
) {
    let Some(mut parser) = Highlighter::new(syntax) else {
        return;
    };
    let mut pending: Option<Request> = None;
    loop {
        let Some(mut request) = pending.take().or_else(|| requests.recv().ok()) else {
            return;
        };
        while let Ok(newer) = requests.try_recv() {
            request = merged(&request, newer);
        }
        if let Some(line) = request.invalid_from.take() {
            parser.invalidate_from(line);
        }
        if !parser.extend(line_of(&request.rope), request.visible.start, STEP) {
            // A step done: look for a newer window before the next one.
            pending = Some(match requests.try_recv() {
                Ok(newer) => merged(&request, newer),
                Err(_) => request,
            });
            continue;
        }
        if let Some(lines) = parser.lines(line_of(&request.rope), request.visible.clone()) {
            let answer = Answer {
                generation: request.generation,
                start: request.visible.start,
                lines,
            };
            if answers.send(answer).is_err() {
                return;
            }
            ctx.request_repaint();
        }
    }
}

/// The newer request, with the earlier edits it has not seen.
fn merged(older: &Request, mut newer: Request) -> Request {
    newer.invalid_from = match (older.invalid_from, newer.invalid_from) {
        (Some(a), Some(b)) => Some(a.min(b)),
        (a, b) => a.or(b),
    };
    newer
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rope(lines: usize) -> text::Rope {
        let text: Vec<String> = (0..lines).map(|n| format!("let a{n} = {n};")).collect();
        text::Rope::from_str(&(text.join("\n") + "\n"))
    }

    /// Frames until `visible` is coloured, the parser's thread working meanwhile.
    fn coloured(colouring: &mut Colouring, rope: &text::Rope, visible: Range<usize>) -> bool {
        let started = std::time::Instant::now();
        while started.elapsed().as_secs() < 20 {
            colouring.window(rope, visible.clone());
            if visible.clone().all(|line| colouring.fresh.contains(&line)) {
                return true;
            }
            std::thread::yield_now();
        }
        false
    }

    #[test]
    fn a_window_far_down_is_coloured_once_the_thread_reaches_it() {
        text::highlight::warm();
        let ctx = egui::Context::default();
        let rope = rope(20_000);
        let mut colouring = Colouring::new(&ctx, "JavaScript").unwrap();

        assert!(coloured(&mut colouring, &rope, 15_000..15_040));
        let line = colouring.line(15_000).unwrap();
        assert!(line.iter().any(|(_, token)| *token == Token::Keyword));
    }

    #[test]
    fn an_edit_keeps_the_old_colours_until_the_new_arrive() {
        text::highlight::warm();
        let ctx = egui::Context::default();
        let before = rope(100);
        let mut colouring = Colouring::new(&ctx, "JavaScript").unwrap();
        assert!(coloured(&mut colouring, &before, 0..50));

        colouring.edited(20);
        assert!(colouring.line(30).is_some(), "drawn as it was meanwhile");
        let after = text::Rope::from_str(&before.to_string().replacen("let a20", "// a20", 1));
        assert!(coloured(&mut colouring, &after, 0..50));
        let line = colouring.line(20).unwrap();
        assert_eq!(line.first().map(|(_, token)| *token), Some(Token::Comment));
    }

    #[test]
    fn a_card_s_lines_are_coloured_once_the_grammars_are_in() {
        text::highlight::warm();
        let ctx = egui::Context::default();
        let lines = short(&ctx, "SQL", "SELECT 1;");
        assert!(lines[0].iter().any(|(_, token)| *token == Token::Keyword));
        assert!(Colouring::new(&ctx, "Not a language").is_none());
    }

    /// The `syntax-tokens` corpus, read as a report (gap E11): how many of the reference's
    /// coloured characters take the same class here. `cargo test -p ui syntax_tokens_report --
    /// --ignored --nocapture`.
    #[test]
    #[ignore = "a report, not a check: TextMate grammars cut tokens otherwise than Lezer"]
    fn syntax_tokens_report() {
        text::highlight::warm();
        let corpus = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../golden/syntax-tokens/cases.json");
        let cases: Vec<serde_json::Value> =
            serde_json::from_str(&std::fs::read_to_string(corpus).unwrap()).unwrap();
        let class = |token: Token| match token {
            Token::Keyword => "tok-keyword",
            Token::String => "tok-string",
            Token::Literal => "tok-literal",
            Token::Name => "tok-name",
            Token::Property => "tok-property",
            Token::Meta => "tok-meta",
            Token::Comment => "tok-comment",
            Token::Deleted => "tok-deleted",
            Token::Emphasis => "tok-emphasis",
            Token::Strong => "tok-strong",
        };
        let (mut same, mut total) = (0, 0);
        for case in &cases {
            let tag = case["input"]["language"].clone();
            let language: engine::notes::language::Language = serde_json::from_value(tag).unwrap();
            let code = case["input"]["text"].as_str().unwrap();
            let chars: Vec<char> = code.chars().collect();
            let mut ours = vec![""; chars.len()];
            if let Some(syntax) = crate::notes::language::syntax(language) {
                let mut at = 0;
                for (line, coloured) in code
                    .split_inclusive('\n')
                    .zip(text::highlight::highlight(code, syntax))
                {
                    for (range, token) in coloured {
                        for slot in &mut ours[at + range.start..at + range.end] {
                            *slot = class(token);
                        }
                    }
                    at += line.chars().count();
                }
            }
            let (mut case_same, mut case_total) = (0, 0);
            for range in case["output"].as_array().unwrap() {
                let from = usize::try_from(range["from"].as_u64().unwrap()).unwrap();
                let to = usize::try_from(range["to"].as_u64().unwrap()).unwrap();
                let wanted = range["classes"].as_str().unwrap();
                for slot in ours.iter().take(to.min(chars.len())).skip(from) {
                    case_total += 1;
                    if slot.contains(wanted) {
                        case_same += 1;
                    }
                }
            }
            println!("{language:?}: {case_same}/{case_total}");
            same += case_same;
            total += case_total;
        }
        println!("all: {same}/{total}");
    }
}
