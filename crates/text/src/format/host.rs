//! Prettier's host: one thread, one QuickJS runtime — a runtime is not shared between threads —
//! Prettier and its nine plugins evaluated once, by the first format. A host that cannot start
//! answers every format `Failed`.

use std::sync::OnceLock;
use std::sync::mpsc::{self, Sender};

use rquickjs::{Array, Context, Ctx, Function, Object, Promise, Runtime, Value};

use super::{Options, Parser};

/// Prettier's own files, as `npm run prettier` copies them: each sets a global.
const SOURCES: [&str; 10] = [
    include_str!("../../prettier/standalone.js"),
    include_str!("../../prettier/angular.js"),
    include_str!("../../prettier/babel.js"),
    include_str!("../../prettier/estree.js"),
    include_str!("../../prettier/graphql.js"),
    include_str!("../../prettier/html.js"),
    include_str!("../../prettier/markdown.js"),
    include_str!("../../prettier/postcss.js"),
    include_str!("../../prettier/typescript.js"),
    include_str!("../../prettier/yaml.js"),
];

/// Its promise settled into a plain object: the text and cursor, or where the syntax broke.
const PRINT: &str =
    "globalThis.__print = (text, options) => prettier.formatWithCursor(text, options).then(
  ({ formatted, cursorOffset }) => ({ text: formatted, cursor: cursorOffset }),
  (error) => ({ line: error?.loc?.start?.line, column: error?.loc?.start?.column }),
);";

/// Prettier with every plugin holds about 30 MB; a large document's tree some more.
const MEMORY: usize = 512 * 1024 * 1024;
/// Its printers recurse deeply: the thread's own stack is larger than what QuickJS may use.
const STACK: usize = 8 * 1024 * 1024;
const THREAD_STACK: usize = 16 * 1024 * 1024;

pub(super) enum Printed {
    /// The cursor in UTF-16 units of the text.
    Text {
        text: String,
        cursor: usize,
    },
    Syntax {
        line: u32,
        column: u32,
    },
    Failed,
}

struct Job {
    text: String,
    cursor: usize,
    parser: Parser,
    options: Options,
    answer: Sender<Printed>,
}

static HOST: OnceLock<Sender<Job>> = OnceLock::new();

/// `cursor` in UTF-16 units of `text`.
pub(super) fn print(text: &str, cursor: usize, parser: Parser, options: Options) -> Printed {
    let (answer, answered) = mpsc::channel();
    let job = Job {
        text: text.to_owned(),
        cursor,
        parser,
        options,
        answer,
    };
    if HOST.get_or_init(start).send(job).is_err() {
        return Printed::Failed;
    }
    answered.recv().unwrap_or(Printed::Failed)
}

fn start() -> Sender<Job> {
    let (jobs, queue) = mpsc::channel::<Job>();
    // Not started, the queue is dropped with the closure, and every send fails.
    let _ = std::thread::Builder::new()
        .name("prettier".to_owned())
        .stack_size(THREAD_STACK)
        .spawn(move || {
            let prettier = Prettier::load();
            for job in queue {
                let printed = prettier
                    .as_ref()
                    .map_or(Printed::Failed, |prettier| prettier.print(&job));
                let _ = job.answer.send(printed);
            }
        });
    jobs
}

struct Prettier {
    // Kept for as long as the context: the context lives in it.
    _runtime: Runtime,
    context: Context,
}

impl Prettier {
    fn load() -> Option<Self> {
        let runtime = Runtime::new().ok()?;
        runtime.set_memory_limit(MEMORY);
        runtime.set_max_stack_size(STACK);
        let context = Context::full(&runtime).ok()?;
        let loaded = context.with(|ctx| -> rquickjs::Result<()> {
            for source in SOURCES {
                ctx.eval::<(), _>(source)?;
            }
            ctx.eval::<(), _>(PRINT)
        });
        loaded.ok()?;
        Some(Self {
            _runtime: runtime,
            context,
        })
    }

    fn print(&self, job: &Job) -> Printed {
        self.context.with(|ctx| {
            let printed = call(&ctx, job).and_then(|promise| promise.finish::<Object<'_>>());
            printed.map_or(Printed::Failed, |printed| read(&printed))
        })
    }
}

fn call<'js>(ctx: &Ctx<'js>, job: &Job) -> rquickjs::Result<Promise<'js>> {
    let globals = ctx.globals();
    let installed: Object<'_> = globals.get("prettierPlugins")?;
    let plugins = Array::new(ctx.clone())?;
    for (at, name) in plugins_of(job.parser).iter().enumerate() {
        plugins.set(at, installed.get::<_, Value<'_>>(*name)?)?;
    }
    let options = Object::new(ctx.clone())?;
    options.set("parser", job.parser.name())?;
    options.set("plugins", plugins)?;
    options.set(
        "cursorOffset",
        f64::from(u32::try_from(job.cursor).unwrap_or(0)),
    )?;
    // Prettier's default is `lf`, which would rewrite a CRLF snippet's every line.
    options.set("endOfLine", "auto")?;
    options.set("printWidth", job.options.print_width)?;
    options.set("tabWidth", job.options.tab_width)?;
    options.set("useTabs", job.options.use_tabs)?;
    options.set("singleQuote", job.options.single_quote)?;
    options.set("semi", job.options.semi)?;
    let trailing = if job.options.trailing_comma {
        "all"
    } else {
        "none"
    };
    options.set("trailingComma", trailing)?;
    let print: Function<'_> = globals.get("__print")?;
    print.call((job.text.as_str(), options))
}

/// The plugins a parser needs: HTML and Vue format their `<script>` and `<style>` too.
fn plugins_of(parser: Parser) -> &'static [&'static str] {
    match parser {
        Parser::Babel | Parser::Json => &["babel", "estree"],
        Parser::Typescript => &["typescript", "estree"],
        Parser::Css | Parser::Scss | Parser::Less => &["postcss"],
        Parser::Html => &["babel", "estree", "html", "postcss"],
        Parser::Vue => &["babel", "estree", "typescript", "html", "postcss"],
        Parser::Angular => &["html", "angular", "postcss"],
        Parser::Markdown => &["markdown"],
        Parser::Yaml => &["yaml"],
        Parser::Graphql => &["graphql"],
    }
}

fn read(printed: &Object<'_>) -> Printed {
    if let Ok(Some(text)) = printed.get::<_, Option<String>>("text") {
        let cursor = printed.get::<_, f64>("cursor").unwrap_or(0.0);
        return Printed::Text {
            text,
            cursor: whole(cursor),
        };
    }
    let place = |key| printed.get::<_, Option<f64>>(key).ok().flatten();
    match (place("line"), place("column")) {
        (Some(line), Some(column)) => Printed::Syntax {
            line: u32::try_from(whole(line)).unwrap_or(0),
            column: u32::try_from(whole(column)).unwrap_or(0),
        },
        _ => Printed::Failed,
    }
}

/// A count JavaScript wrote as a number; a negative one — no cursor found — is the start.
fn whole(value: f64) -> usize {
    if value.is_finite() && value > 0.0 {
        #[expect(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "positive and finite, an offset into a string"
        )]
        let whole = value as usize;
        whole
    } else {
        0
    }
}
