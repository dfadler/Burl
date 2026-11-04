//! The hover text box on a hand-built input (no GL): shadow, coloured box, white lines.

use burl_render::*;
use glam::{Vec2, Vec3};

#[test]
fn box_is_a_shadow_then_the_half_bright_colour_then_the_lines() {
    let mut mgr = FontManager::new(FREE_SANS, 100).unwrap();
    let mut tb = TextBox::new(&mut mgr, 18);
    let mut out = Vec::new();
    let lines = ["main.rs".to_string(), "src/".to_string()];
    tb.draw(
        &mut mgr,
        &mut out,
        &lines,
        Vec3::new(1.0, 0.5, 0.0),
        Vec2::new(300.0, 200.0),
        Vec2::new(640.0, 480.0),
    );
    let colour = |b: &Batch| match &b.vertices {
        Vertices::Quads(v) => v[0].colour,
        _ => unreachable!(),
    };
    assert_eq!(out[0].offset, [3.0, 3.0]);
    assert_eq!(colour(&out[0]), [0.0, 0.0, 0.0, 0.333]);
    assert_eq!(out[1].offset, [0.0, 0.0]);
    assert_eq!(colour(&out[1]), [0.5, 0.25, 0.0, 1.0]);
    // two lines of text follow: 22 pixels per row, the box is 2 + 2 * 22 high above the cursor
    assert!(out.len() > 2);
    let Vertices::Quads(q) = &out[1].vertices else {
        unreachable!()
    };
    assert_eq!(q[0].pos[1], 200.0 - 46.0);
    assert_eq!(q[2].pos[1] - q[0].pos[1], 46.0);
}
