//! The diagrams a Note really holds (ADR S7): the specs', scenario 03's and every `mermaid` block
//! of the migration plan, each drawn. Mermaid's own pictures are the visual reference's
//! screenshots (`golden/README.md`): what differs from them is E6, judged by eye.

use diagram::{Drawn, Fonts, Theme, draw};

const INTER: &[u8] = include_bytes!("../../ui/assets/fonts/inter-latin-wght-normal.ttf");
const PLAN: &str = include_str!("../../../docs/egui-migration-plan.md");

const SPECS: [&str; 6] = [
    "graph TD; A-->B",
    "sequenceDiagram\n  Alice->>Bob: POST /login\n  Bob-->>Alice: 200 OK",
    "classDiagram\n  class Repository~T~ {\n    +find(id) T\n    +all() Vec~T~\n  }\n  Repository~T~ \"1\" --> \"*\" Entity",
    "stateDiagram-v2\n  [*] --> Draft\n  Draft --> Saved: blur\n  Saved --> Draft: edit\n  Saved --> [*]",
    "pie title Notes\n  \"Snippets\" : 42\n  \"Notes\" : 18\n  \"Lists\" : 7",
    "flowchart LR\n  subgraph Core\n    A[engine] --> B[(SQLite)]\n  end\n  UI ==> A\n  style A stroke-width:3px",
];

fn plan_diagrams() -> Vec<&'static str> {
    PLAN.split("```mermaid\n")
        .skip(1)
        .filter_map(|block| block.split("\n```").next())
        .collect()
}

#[test]
fn every_diagram_of_the_corpus_is_drawn() {
    let fonts = Fonts::new(&[INTER]).unwrap();
    let theme = Theme {
        dark: false,
        background: "#ffffff".into(),
        primary: "#eeeae2".into(),
        primary_text: "#1f1c17".into(),
        primary_border: "#d6d0c4".into(),
        secondary: "#f6f3ed".into(),
        line: "#6b655a".into(),
        text: "#1f1c17".into(),
    };
    let plan = plan_diagrams();
    assert!(plan.len() >= 7, "the plan's diagrams: {}", plan.len());

    let undrawn: Vec<String> = SPECS
        .iter()
        .chain(&plan)
        .filter_map(|source| match draw(source, &theme, &fonts) {
            Drawn::Picture { .. } => None,
            other => Some(format!(
                "{}: {other:?}",
                source.lines().next().unwrap_or("")
            )),
        })
        .collect();

    assert!(undrawn.is_empty(), "{undrawn:#?}");
}
