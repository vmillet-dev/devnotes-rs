//! Spike S7: native Mermaid engines on the diagrams of the specs, the e2e scenario and the plan.
//! Writes every SVG and a PNG (resvg, the plan's raster path) to ../out/s7/<engine>/.

use std::time::Instant;

type Engine = (&'static str, fn(&str) -> Result<String, String>);

fn merman(source: &str) -> Result<String, String> {
    use merman::{OperationControl, RenderOutput, RenderRequest, Renderer, SvgRequest};
    let request = SvgRequest { pipeline: Some(merman::svg::SvgPipeline::resvg_safe()), ..SvgRequest::default() };
    match Renderer::new().render(RenderRequest::svg(source, OperationControl::new(), request)) {
        Ok(RenderOutput::Svg(Some(svg))) => Ok(svg.svg().to_string()),
        Ok(_) => Err("no diagram detected".into()),
        Err(error) => Err(format!("{error}")),
    }
}

fn mmdr(source: &str) -> Result<String, String> {
    mermaid_rs_renderer::render(source).map_err(|e| format!("{e:#}"))
}

fn mermaid_svg(source: &str) -> Result<String, String> {
    mermaid_svg::render(source).map_err(|e| format!("{e}"))
}

fn rasterise(svg: &str, fonts: &resvg::usvg::fontdb::Database) -> Result<Vec<u8>, String> {
    let mut options = resvg::usvg::Options::default();
    options.fontdb = std::sync::Arc::new(fonts.clone());
    let tree = resvg::usvg::Tree::from_str(svg, &options).map_err(|e| e.to_string())?;
    let size = tree.size().to_int_size();
    let mut pixmap = resvg::tiny_skia::Pixmap::new(size.width().max(1), size.height().max(1)).ok_or("empty")?;
    pixmap.fill(resvg::tiny_skia::Color::WHITE);
    resvg::render(&tree, resvg::tiny_skia::Transform::default(), &mut pixmap.as_mut());
    pixmap.encode_png().map_err(|e| e.to_string())
}

fn main() {
    let engines: [Engine; 3] = [("merman", merman), ("mermaid-rs-renderer", mmdr), ("mermaid-svg", mermaid_svg)];
    let mut fonts = resvg::usvg::fontdb::Database::new();
    fonts.load_system_fonts();
    let mut names: Vec<_> = std::fs::read_dir("diagrams").expect("diagrams").filter_map(|e| e.ok()).map(|e| e.file_name().into_string().expect("name")).collect();
    names.sort();
    for (engine, render) in engines {
        let dir = format!("../out/s7/{engine}");
        std::fs::create_dir_all(&dir).expect("out");
        println!("== {engine}");
        for name in &names {
            let source = std::fs::read_to_string(format!("diagrams/{name}")).expect("diagram");
            let started = Instant::now();
            let result = std::panic::catch_unwind(|| render(&source)).unwrap_or_else(|_| Err("PANIC".into()));
            let ms = started.elapsed().as_secs_f64() * 1e3;
            match result {
                Ok(svg) => {
                    std::fs::write(format!("{dir}/{name}.svg"), &svg).expect("write");
                    let png = rasterise(&svg, &fonts);
                    if let Ok(png) = &png {
                        std::fs::write(format!("{dir}/{name}.png"), png).expect("write");
                    }
                    let has_label = !svg.contains("<foreignObject") ;
                    println!("  {name:<22} ok   {ms:7.1} ms  {:>6} B svg  resvg: {:<5} no foreignObject: {has_label}", svg.len(), png.is_ok());
                }
                Err(error) => println!("  {name:<22} ERR  {ms:7.1} ms  {}", error.lines().take(3).collect::<Vec<_>>().join(" | ")),
            }
        }
    }
}
