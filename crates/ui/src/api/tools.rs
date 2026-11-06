//! The tools' calls: pure functions, each on the `compute` queue.

/// How the home's search reads a text: the core's fold, called directly (D12).
pub fn fold(text: &str) -> String {
    engine::notes::view::fold(text)
}
