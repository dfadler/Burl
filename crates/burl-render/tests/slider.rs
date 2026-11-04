//! The position slider (no GL): hit-testing, the click-to-percent mapping, the fade and the
//! batches it draws. `BURL_BLESS=1 cargo test -p burl-render --test slider` rewrites the snapshot.

use burl_render::*;
use glam::Vec2;

fn rig(show: bool) -> (FontManager, Slider) {
    let mut mgr = FontManager::new(FREE_SANS, 100).unwrap();
    let s = Slider::new(&mut mgr, 1.0, 640.0, 480.0, show);
    (mgr, s)
}

fn draw(mgr: &mut FontManager, s: &mut Slider) -> Vec<Batch> {
    let mut out = vec![];
    s.draw(mgr, &mut out, 640.0);
    out
}

/// Bounds are (35, 410)-(605, 445) for 640x480; the percent is the x fraction, edges count.
#[test]
fn hit_test_and_percent() {
    let (_, s) = rig(true);
    assert_eq!(s.percent_at(Vec2::new(35.0, 410.0)), Some(0.0));
    assert_eq!(s.percent_at(Vec2::new(605.0, 445.0)), Some(1.0));
    assert_eq!(s.percent_at(Vec2::new(320.0, 430.0)), Some(285.0 / 570.0));
    for p in [
        (34.9, 430.0),
        (605.1, 430.0),
        (320.0, 409.9),
        (320.0, 445.1),
    ] {
        assert_eq!(s.percent_at(Vec2::new(p.0, p.1)), None, "{p:?}");
    }
}

#[test]
fn resize_moves_the_bounds() {
    let (_, mut s) = rig(true);
    s.resize(1000.0, 800.0);
    assert_eq!(s.percent_at(Vec2::new(500.0, 740.0)), Some(465.0 / 930.0));
    assert_eq!(s.percent_at(Vec2::new(500.0, 430.0)), None);
}

#[test]
fn click_seeks_only_on_the_slider() {
    let (_, mut s) = rig(true);
    assert_eq!(s.click(Vec2::new(320.0, 100.0)), None);
    assert_eq!(s.click(Vec2::new(605.0, 430.0)), Some(1.0));
}

/// `show()` starts it fading in; a hidden one stays invisible until the mouse is over it, then
/// fades out a second after the mouse leaves.
#[test]
fn fade() {
    let (mut mgr, mut s) = rig(false);
    s.logic(0.5);
    assert!(draw(&mut mgr, &mut s).is_empty());
    s.mouse_over(Vec2::new(100.0, 430.0));
    s.logic(0.5);
    let alpha = |b: &[Batch]| match &b[0].vertices {
        Vertices::Quads(v) => v[0].colour[3],
        _ => panic!(),
    };
    // the first batch is the shadow: 0.7 * alpha
    assert_eq!(alpha(&draw(&mut mgr, &mut s)), 0.35);
    s.mouse_over(Vec2::new(100.0, 100.0));
    s.logic(0.6);
    s.logic(0.6);
    s.logic(0.6);
    assert!(draw(&mut mgr, &mut s).is_empty());
}

#[test]
fn slider_matches_snapshot() {
    let (mut mgr, mut s) = rig(true);
    s.set_percent(0.25);
    s.logic(0.5);
    s.mouse_over(Vec2::new(200.0, 430.0));
    s.set_caption(&mut mgr, "Monday, 01 January, 2024");
    let mut got = String::new();
    for b in draw(&mut mgr, &mut s) {
        got += &format!(
            "{:?} {:?} off={:?} tex={:?}\n",
            b.pass,
            b.pipeline,
            b.offset,
            b.texture.map(|t| t.0)
        );
        if let Vertices::Quads(v) = &b.vertices {
            for v in v {
                got += &format!("  {:?} {:?}\n", v.pos, v.colour);
            }
        }
    }
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/snapshots/slider.txt");
    if matches!(std::env::var("BURL_BLESS").as_deref(), Ok("1")) {
        std::fs::write(path, &got).unwrap();
    }
    assert_eq!(got, std::fs::read_to_string(path).unwrap());
}
