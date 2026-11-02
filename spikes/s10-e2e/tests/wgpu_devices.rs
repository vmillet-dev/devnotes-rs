//! S10: several `wgpu` devices in one test process on a software driver, and the rule for
//! running such tests concurrently. Each harness renders through its own `wgpu` device.

use egui_kittest::Harness;

fn harness(label: &'static str) -> Harness<'static> {
    Harness::builder().with_size(egui::vec2(400.0, 300.0)).wgpu().build_ui(move |ui| {
        ui.heading(label);
        for n in 0..30 {
            ui.label(format!("{label} {n}"));
        }
    })
}

fn render_many(label: &'static str) {
    let mut harness = harness(label);
    for _ in 0..20 {
        harness.step();
        let image = harness.render().expect("a rendered frame");
        assert_eq!(image.width(), 400);
    }
}

#[test]
fn two_devices_on_one_thread() {
    let mut first = harness("first");
    let mut second = harness("second");
    for _ in 0..10 {
        first.render().expect("first");
        second.render().expect("second");
    }
}

#[test]
fn two_devices_on_two_threads_at_once() {
    let a = std::thread::spawn(|| render_many("a"));
    let b = std::thread::spawn(|| render_many("b"));
    a.join().expect("a");
    b.join().expect("b");
}

#[test]
fn parallel_1() { render_many("p1"); }
#[test]
fn parallel_2() { render_many("p2"); }
#[test]
fn parallel_3() { render_many("p3"); }
#[test]
fn parallel_4() { render_many("p4"); }
#[test]
fn parallel_5() { render_many("p5"); }
#[test]
fn parallel_6() { render_many("p6"); }
