//! S8: the native formatters the plan names, on the same fixtures, compared with Prettier's
//! output (fixtures/golden, written by golden.mjs). Run from spikes/s8-format.

use std::path::Path;
use std::time::Instant;

fn typescript(text: &str, extension: &str) -> anyhow::Result<String> {
    use dprint_plugin_typescript::configuration::{ConfigurationBuilder, QuoteStyle, SemiColons, TrailingCommas};
    let config = ConfigurationBuilder::new()
        .line_width(100)
        .indent_width(2)
        .quote_style(QuoteStyle::PreferSingle)
        .semi_colons(SemiColons::Always)
        .trailing_commas(TrailingCommas::OnlyMultiLine)
        .build();
    let path = format!("snippet.{extension}");
    let out = dprint_plugin_typescript::format_text(dprint_plugin_typescript::FormatTextOptions {
        path: Path::new(&path),
        extension: None,
        text: text.to_string(),
        config: &config,
        external_formatter: None,
    })?;
    Ok(out.unwrap_or_else(|| text.to_string()))
}

fn json(text: &str) -> anyhow::Result<String> {
    let config = dprint_plugin_json::configuration::ConfigurationBuilder::new().line_width(100).build();
    Ok(dprint_plugin_json::format_text(Path::new("a.json"), text, &config)?.unwrap_or_else(|| text.to_string()))
}

fn css(text: &str, syntax: malva::Syntax) -> anyhow::Result<String> {
    let mut options = malva::config::FormatOptions::default();
    options.layout.print_width = 100;
    Ok(malva::format_text(text, syntax, &options).map_err(|e| anyhow::anyhow!("{e:?}"))?)
}

fn yaml(text: &str) -> anyhow::Result<String> {
    let mut options = pretty_yaml::config::FormatOptions::default();
    options.layout.print_width = 100;
    Ok(pretty_yaml::format_text(text, &options).map_err(|e| anyhow::anyhow!("{e:?}"))?)
}

fn graphql(text: &str) -> anyhow::Result<String> {
    let mut options = pretty_graphql::config::FormatOptions::default();
    options.layout.print_width = 100;
    Ok(pretty_graphql::format_text(text, &options).map_err(|e| anyhow::anyhow!("{e:?}"))?)
}

fn markdown(text: &str) -> anyhow::Result<String> {
    let config = dprint_plugin_markdown::configuration::ConfigurationBuilder::new().line_width(100).build();
    Ok(dprint_plugin_markdown::format_text(text, &config, |_, _, _| Ok(None))?.unwrap_or_else(|| text.to_string()))
}

fn markup(text: &str, language: markup_fmt::Language) -> anyhow::Result<String> {
    let mut options = markup_fmt::config::FormatOptions::default();
    options.layout.print_width = 100;
    markup_fmt::format_text(text, language, &options, |code, _| Ok::<_, anyhow::Error>(code.into()))
        .map_err(|e| anyhow::anyhow!("{e:?}"))
}

fn format(name: &str, text: &str) -> anyhow::Result<String> {
    match name.rsplit('.').next().expect("ext") {
        "ts" => typescript(text, "ts"),
        "js" => typescript(text, "js"),
        "json" => json(text),
        "css" => css(text, malva::Syntax::Css),
        "scss" => css(text, malva::Syntax::Scss),
        "less" => css(text, malva::Syntax::Less),
        "yaml" => yaml(text),
        "graphql" => graphql(text),
        "md" => markdown(text),
        "html" => markup(text, markup_fmt::Language::Html),
        "vue" => markup(text, markup_fmt::Language::Vue),
        "angular" => markup(text, markup_fmt::Language::Angular),
        other => anyhow::bail!("no formatter for {other}"),
    }
}

fn main() {
    let mut names: Vec<_> = std::fs::read_dir("fixtures").expect("fixtures").filter_map(|e| e.ok()).map(|e| e.file_name().into_string().expect("name")).filter(|n| n.contains('.')).collect();
    names.sort();
    std::fs::create_dir_all("../out/s8-native").expect("out");
    for name in names {
        let text = std::fs::read_to_string(format!("fixtures/{name}")).expect("fixture");
        let golden = std::fs::read_to_string(format!("fixtures/golden/{name}")).expect("golden");
        let mut samples = Vec::new();
        let mut out = String::new();
        for _ in 0..20 {
            let started = Instant::now();
            out = format(&name, &text).unwrap_or_else(|e| format!("ERROR {e}"));
            samples.push(started.elapsed().as_secs_f64() * 1e3);
        }
        samples.sort_by(f64::total_cmp);
        let differing = out.lines().zip(golden.lines()).filter(|(a, b)| a != b).count() + out.lines().count().abs_diff(golden.lines().count());
        std::fs::write(format!("../out/s8-native/{name}"), &out).expect("write");
        println!("{name:<18} {:>6} B  p50 {:6.2}  p95 {:6.2} ms  identical: {:<5}  lines differing: {differing}/{}", text.len(), samples[10], samples[19], out == golden, golden.lines().count());
    }
}
