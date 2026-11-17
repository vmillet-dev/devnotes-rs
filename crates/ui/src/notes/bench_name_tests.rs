//! What a screen reader says of the notes area's regions: words, not test ids.

use egui_kittest::kittest::{NodeT, Queryable};

use crate::notes::bench::area;
use crate::testing;

#[test]
fn the_regions_are_named_in_words_not_by_their_ids() {
    let (backend, profile) = testing::unlocked();
    testing::note(&backend, "psql prod");
    let harness = area(backend, profile);
    for id in ["area-notes", "canvas"] {
        let node = harness.get_by(|node| node.author_id() == Some(id));
        let name = node.accesskit_node().label().unwrap_or_default();
        assert!(!name.is_empty() && name != id, "{id} is read « {name} »");
    }
}
