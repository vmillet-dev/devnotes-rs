//! A stack overflow stops the whole test binary: this file is a binary of its own.

/// An imported Note can hold any depth of quotes: it opens, and writes back, without a crash.
#[test]
fn a_note_quoted_ten_thousand_deep_opens_and_writes_back() {
    let text = "> ".repeat(10_000) + "deep";

    let document = richtext::read(&text);
    let written = richtext::write(&document);

    assert!(written.contains("deep"));
}
