//! Text engine regression layer: FXFont placement, glyph packing and the batch order, with no GL.
//! `BURL_BLESS=1 cargo test -p burl-render` rewrites the snapshot.

use burl_render::*;
use glam::Vec4;

fn describe(batches: &[Batch]) -> String {
    let mut s = String::new();
    for b in batches {
        s += &format!(
            "{:?} {:?} {:?} {:?} tex={:?}\n",
            b.pass,
            b.pipeline,
            b.blend,
            b.space,
            b.texture.map(|t| t.0)
        );
        if let Vertices::Quads(v) = &b.vertices {
            for v in v {
                s += &format!("  {:?} {:?} {:?}\n", v.pos, v.colour, v.uv);
            }
        }
    }
    s
}

fn manager() -> FontManager {
    FontManager::new(FREE_SANS, 100).unwrap()
}

#[test]
fn text_batches_match_snapshot() {
    let mut m = manager();
    let mut f = m.grab(16);
    f.shadow = true;
    f.round = true;
    f.set_colour(Vec4::new(1.0, 0.5, 0.25, 0.5));

    // Immediate mode: shadow batch, then colour batch.
    let mut frame = Frame::default();
    m.draw(&f, &mut frame.overlay, 10.0, 20.0, "Ag\t.");
    // Buffered (dir/user names): colour only, shadow comes from the text shader.
    m.start_buffer();
    m.draw_buffered(&f, 30.4, 40.6, "Ag");
    frame.names = std::mem::take(&mut m.buffer);

    let got = describe(&frame.batches());
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/snapshots/text_frame.txt"
    );
    if matches!(std::env::var("BURL_BLESS").as_deref(), Ok("1")) {
        std::fs::write(path, &got).unwrap();
    }
    assert_eq!(got, std::fs::read_to_string(path).unwrap());
}

#[test]
fn width_is_the_sum_of_whole_pixel_advances() {
    let mut m = manager();
    let f = m.grab(14);
    let (a, b) = (m.width(&f, "i"), m.width(&f, "W"));
    assert_eq!(m.width(&f, "iW"), a + b);
    assert_eq!(a, a.round());
    assert!(b > a && a > 0.0);
    assert_eq!(m.width(&f, ""), 0.0);
}

#[test]
fn right_alignment_ends_the_text_at_x() {
    let mut m = manager();
    let mut f = m.grab(14);
    f.align_top = false;
    m.start_buffer();
    m.draw_buffered(&f, 200.0, 50.0, "ab");
    let left = m.buffer.vertices()[0].pos[0];
    f.align_right = true;
    m.start_buffer();
    m.draw_buffered(&f, 200.0, 50.0, "ab");
    let right = m.buffer.vertices()[0].pos[0];
    assert_eq!(left - right, m.width(&f, "ab"));
}

#[test]
fn precached_pages_upload_once() {
    let mut m = manager();
    let f = m.grab(14);
    let up = m.take_updates();
    assert_eq!(up.len(), 1);
    assert_eq!(up[0].id, TextureId(100));
    assert_eq!(up[0].rgba.len(), PAGE_SIZE * PAGE_SIZE * 4);
    assert!(
        up[0].rgba.chunks(4).any(|p| p[3] > 0),
        "glyph coverage was rasterised"
    );
    assert!(
        up[0].rgba.chunks(4).any(|p| (1..255).contains(&p[3])),
        "anti-aliased edges keep partial coverage"
    );
    assert!(m.take_updates().is_empty());
    // A glyph outside the precache list dirties the page again.
    m.width(&f, "é");
    assert_eq!(m.take_updates().len(), 1);
}

#[test]
fn height_is_ascender_minus_descender_ceiled() {
    let mut m = manager();
    let f = m.grab(14);
    let h = m.height(&f);
    assert_eq!(h, h.ceil());
    assert!((10.0..=16.0).contains(&h), "{h}");
}
