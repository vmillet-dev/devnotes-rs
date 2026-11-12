//! The response's Tests tab: how many passed, then each test under whose it is.

use egui::{CornerRadius, Rect, Sense, Ui, Vec2, pos2};
use engine::http::scripts::model::{ScriptLevel, TestResult};
use i18n::keys;

use crate::theme::icons::{self, Icon};
use crate::theme::palette;
use crate::widgets::{self, Button};

const BAR: f32 = 6.0;

fn count(tests: &[TestResult], holds: impl Fn(&TestResult) -> bool) -> u32 {
    let held = tests.iter().filter(|test| holds(test)).count();
    u32::try_from(held).unwrap_or(u32::MAX)
}

fn failing(test: &TestResult) -> bool {
    !test.passed && !test.skipped
}

/// The test whose line was asked for, when `openable`: not from the history, whose request may
/// have changed since.
pub(super) fn show<'a>(
    ui: &mut Ui,
    tests: &'a [TestResult],
    (request, openable): (&str, bool),
    language: i18n::Language,
) -> Option<&'a TestResult> {
    let passed = count(tests, |test| test.passed);
    let failed = count(tests, failing);
    summary(ui, passed, failed, language);
    let mut opened = None;
    egui::ScrollArea::vertical()
        .id_salt("http-response-tests")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 6.0;
            let mut group = None;
            for test in tests {
                let holder = (test.level, test.owner.as_deref());
                if group != Some(holder) {
                    group = Some(holder);
                    heading(ui, test, request, language);
                }
                if row(ui, test, openable, language) {
                    opened = Some(test);
                }
            }
        });
    opened
}

fn summary(ui: &mut Ui, passed: u32, failed: u32, language: i18n::Language) {
    let p = palette(ui.ctx());
    let total = passed + failed;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 10.0;
        let (words, hue, author_id) = if failed > 0 {
            let words = i18n::translate(
                language,
                keys::http::tests::FAILED,
                &[("count", failed.into())],
            );
            (words, p.red, "http-tests-failed")
        } else {
            let words = i18n::translate(language, keys::http::tests::ALL_PASSED, &[]);
            (words, p.green, "http-tests-passed")
        };
        let words = egui::RichText::new(words).size(12.5).strong().color(hue);
        let said = ui.add(egui::Label::new(words));
        widgets::address(ui, &said, author_id);
        let ratio = i18n::translate(
            language,
            keys::http::tests::RATIO,
            &[("passed", passed.into()), ("total", total.into())],
        );
        let ratio = egui::RichText::new(ratio).size(12.0).color(p.text_2);
        let said = ui.add(egui::Label::new(ratio));
        widgets::address(ui, &said, "http-tests-ratio");
    });
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), BAR), Sense::hover());
    ui.painter()
        .rect_filled(rect, CornerRadius::same(3), p.red.gamma_multiply(0.5));
    if total > 0 {
        let share = f32::from(u16::try_from(passed).unwrap_or(u16::MAX))
            / f32::from(u16::try_from(total).unwrap_or(u16::MAX));
        let right = rect.left() + rect.width() * share;
        let done = Rect::from_min_max(rect.min, pos2(right, rect.bottom()));
        ui.painter()
            .rect_filled(done, CornerRadius::same(3), p.green);
    }
}

fn heading(ui: &mut Ui, test: &TestResult, request: &str, language: i18n::Language) {
    let p = palette(ui.ctx());
    let holder = test.holder.as_deref().unwrap_or_default();
    let (key, name) = match test.level.unwrap_or_default() {
        ScriptLevel::Request => (keys::http::tests::groups::REQUEST, request),
        ScriptLevel::Folder => (keys::http::tests::groups::FOLDER, holder),
        ScriptLevel::Collection => (keys::http::tests::groups::COLLECTION, holder),
    };
    let words = i18n::translate(language, key, &[("name", name.into())]);
    let words = egui::RichText::new(words)
        .size(12.0)
        .strong()
        .color(p.text_2);
    let said = ui.add(egui::Label::new(words));
    widgets::address(ui, &said, "http-tests-group");
}

/// `true` on its line's button.
fn row(ui: &mut Ui, test: &TestResult, openable: bool, language: i18n::Language) -> bool {
    let p = palette(ui.ctx());
    let tr = |key| i18n::translate(language, key, &[]);
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        let (rect, _) = ui.allocate_exact_size(Vec2::splat(14.0), Sense::hover());
        if test.skipped {
            let mark = Rect::from_center_size(rect.center(), Vec2::new(8.0, 1.5));
            ui.painter().rect_filled(mark, CornerRadius::ZERO, p.text_2);
        } else if test.passed {
            icons::paint(ui, rect, Icon::Check, p.green);
        } else {
            icons::paint(ui, rect, Icon::X, p.red);
        }
        let ink = if test.skipped { p.text_2 } else { p.text_0 };
        let words = egui::RichText::new(&test.name).size(12.5).color(ink);
        let said = ui.add(egui::Label::new(words));
        widgets::address(ui, &said, "http-test");
    });
    if !failing(test) {
        return false;
    }
    ui.indent("http-test-why", |ui| {
        if test.expected.is_some() || test.actual.is_some() {
            let dash = "—".to_owned();
            let words = format!(
                "{} {}   {} {}",
                tr(keys::http::tests::EXPECTED),
                test.expected.as_ref().unwrap_or(&dash),
                tr(keys::http::tests::ACTUAL),
                test.actual.as_ref().unwrap_or(&dash),
            );
            let words = egui::RichText::new(words)
                .monospace()
                .size(12.0)
                .color(p.text_1);
            let said = ui.add(egui::Label::new(words).wrap());
            widgets::address(ui, &said, "http-test-versus");
        } else if let Some(message) = &test.message {
            let words = egui::RichText::new(message).size(12.0).color(p.red);
            let said = ui.add(egui::Label::new(words).wrap());
            widgets::address(ui, &said, "http-test-message");
        }
        let mut opened = false;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            if let Some(code) = &test.code {
                let words = egui::RichText::new(code)
                    .monospace()
                    .size(12.0)
                    .color(p.text_2);
                let said = ui.add(egui::Label::new(words).truncate());
                widgets::address(ui, &said, "http-test-code");
            }
            if let Some(line) = test.line.filter(|_| openable) {
                let words =
                    i18n::translate(language, keys::http::tests::LINE, &[("line", line.into())]);
                opened = Button::new(&words, "http-test-line")
                    .kind(widgets::Kind::Quiet)
                    .show(ui)
                    .clicked();
            }
        });
        opened
    })
    .inner
}

#[cfg(test)]
mod tests {
    use egui_kittest::Harness;
    use egui_kittest::kittest::Queryable;
    use i18n::Language;

    use super::*;

    fn test(name: &str) -> TestResult {
        TestResult {
            name: name.to_owned(),
            passed: true,
            level: Some(ScriptLevel::Request),
            ..TestResult::default()
        }
    }

    fn failed(name: &str) -> TestResult {
        TestResult {
            passed: false,
            line: Some(3),
            code: Some("expect(res.status).toBe(201);".to_owned()),
            ..test(name)
        }
    }

    fn results() -> Vec<TestResult> {
        vec![
            test("the status is 200"),
            TestResult {
                expected: Some("201".to_owned()),
                actual: Some("200".to_owned()),
                ..failed("it was created")
            },
            TestResult {
                message: Some("boom".to_owned()),
                ..failed("it threw")
            },
            TestResult {
                passed: false,
                skipped: true,
                ..test("later")
            },
            TestResult {
                level: Some(ScriptLevel::Folder),
                holder: Some("Auth".to_owned()),
                owner: Some("f1".to_owned()),
                ..test("the folder's own")
            },
            TestResult {
                level: Some(ScriptLevel::Collection),
                holder: Some("Local".to_owned()),
                owner: Some("c1".to_owned()),
                ..test("the collection's own")
            },
        ]
    }

    fn drawn(tests: Vec<TestResult>, openable: bool) -> Harness<'static, Option<String>> {
        let mut harness = crate::testing::harness()
            .with_size(egui::vec2(640.0, 480.0))
            .build_ui_state(
                move |ui, opened: &mut Option<String>| {
                    let asked = show(ui, &tests, ("Ping", openable), Language::En);
                    if let Some(test) = asked {
                        *opened = Some(test.name.clone());
                    }
                },
                None,
            );
        harness.run();
        harness
    }

    fn count(harness: &Harness<'_, Option<String>>, author_id: &str) -> usize {
        harness
            .query_all_by(|node| node.author_id() == Some(author_id))
            .count()
    }

    #[test]
    fn each_test_stands_under_whose_it_is_and_says_why_it_failed() {
        let harness = drawn(results(), true);
        assert_eq!(count(&harness, "http-tests-failed"), 1);
        assert_eq!(count(&harness, "http-tests-group"), 3);
        assert_eq!(count(&harness, "http-test"), 6);
        assert_eq!(count(&harness, "http-test-versus"), 1);
        assert_eq!(count(&harness, "http-test-message"), 1);
        assert_eq!(count(&harness, "http-test-code"), 2);
    }

    #[test]
    fn all_passed_says_so() {
        let harness = drawn(vec![test("the status is 200")], true);
        assert_eq!(count(&harness, "http-tests-passed"), 1);
        assert_eq!(count(&harness, "http-tests-failed"), 0);
    }

    #[test]
    fn a_failed_test_leads_to_its_line_unless_the_history_shows_it() {
        let tests = vec![failed("it was created")];
        let mut harness = drawn(tests.clone(), true);
        harness
            .get_by(|node| node.author_id() == Some("http-test-line"))
            .click();
        harness.run();
        assert_eq!(harness.state().as_deref(), Some("it was created"));

        let harness = drawn(tests, false);
        assert_eq!(count(&harness, "http-test-line"), 0);
    }
}
