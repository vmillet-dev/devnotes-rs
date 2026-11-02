//! S8: Prettier 3.9.10 standalone inside QuickJS (rquickjs, as the script host uses), warm host.
//! `prettier_qjs <prettier package dir>`; run from spikes/s8-format.

use std::time::Instant;

use rquickjs::{Context, Runtime};

fn main() {
    let dir = std::env::args().nth(1).expect("prettier package dir");
    let runtime = Runtime::new().expect("runtime");
    runtime.set_max_stack_size(8 * 1024 * 1024);
    let context = Context::full(&runtime).expect("context");
    let started = Instant::now();
    context.with(|ctx| {
        for file in ["standalone.js", "plugins/estree.js", "plugins/babel.js", "plugins/typescript.js", "plugins/postcss.js", "plugins/yaml.js", "plugins/markdown.js", "plugins/html.js", "plugins/angular.js", "plugins/graphql.js"] {
            let source = std::fs::read_to_string(format!("{dir}/{file}")).expect("plugin");
            ctx.eval::<(), _>(source).unwrap_or_else(|e| panic!("{file}: {e:?}"));
        }
    });
    println!("cold: Prettier + 9 plugins evaluated from source in {:.0} ms", started.elapsed().as_secs_f64() * 1e3);

    let format = |text: &str, parser: &str| -> (f64, String) {
        let started = Instant::now();
        context.with(|ctx| {
            ctx.globals().set("__in", text).expect("set");
            ctx.globals().set("__parser", parser).expect("set");
            ctx.eval::<(), _>(r#"
                globalThis.__out = undefined;
                prettier.format(__in, { parser: __parser, plugins: Object.values(prettierPlugins),
                    printWidth: 100, tabWidth: 2, useTabs: false, singleQuote: true, semi: true,
                    trailingComma: 'all', endOfLine: 'auto' })
                  .then((r) => { globalThis.__out = r; }, (e) => { globalThis.__out = 'ERROR ' + e.message.split('\n')[0]; });
            "#).expect("format called");
        });
        while runtime.execute_pending_job().expect("job") {}
        let out = context.with(|ctx| ctx.globals().get::<_, String>("__out").expect("an answer"));
        (started.elapsed().as_secs_f64() * 1e3, out)
    };

    let parsers = [("ts", "typescript"), ("js", "babel"), ("json", "json"), ("css", "css"), ("scss", "scss"), ("less", "less"), ("html", "html"), ("vue", "vue"), ("md", "markdown"), ("yaml", "yaml"), ("graphql", "graphql"), ("angular", "angular")];
    let mut names: Vec<_> = std::fs::read_dir("fixtures").expect("fixtures").filter_map(|e| e.ok()).map(|e| e.file_name().into_string().expect("name")).filter(|n| n.contains('.')).collect();
    names.sort();
    for name in names {
        let extension = name.rsplit('.').next().expect("ext");
        let parser = parsers.iter().find(|(e, _)| *e == extension).expect("parser").1;
        let text = std::fs::read_to_string(format!("fixtures/{name}")).expect("fixture");
        let golden = std::fs::read_to_string(format!("fixtures/golden/{name}")).expect("golden");
        let (first, out) = format(&text, parser);
        let mut samples: Vec<f64> = (0..20).map(|_| format(&text, parser).0).collect();
        samples.sort_by(f64::total_cmp);
        println!("{name:<18} {:>6} B  first {first:7.1} ms  warm p50 {:7.1}  p95 {:7.1} ms  identical to node: {}", text.len(), samples[10], samples[19], out == golden);
    }
    let status = std::fs::read_to_string("/proc/self/status").expect("status");
    println!("{}", status.lines().filter(|l| l.starts_with("VmHWM") || l.starts_with("VmRSS")).collect::<Vec<_>>().join("  "));
}
