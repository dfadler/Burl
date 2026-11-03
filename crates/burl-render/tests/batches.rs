//! The GL-free render regression layer (#19 item 4): a synthetic frame's batches are rendered to
//! text and compared with a committed snapshot. `BURL_BLESS=1 cargo test -p burl-render` rewrites it.

use burl_render::*;
use glam::{Vec2, Vec4};

fn describe(batches: &[Batch]) -> String {
    let mut s = String::new();
    for b in batches {
        s += &format!(
            "{:?} {:?} {:?} {:?} offset={:?} tex={:?}\n",
            b.pass,
            b.pipeline,
            b.blend,
            b.space,
            b.offset,
            b.texture.map(|t| t.0)
        );
        match &b.vertices {
            Vertices::Quads(v) => v
                .iter()
                .for_each(|v| s += &format!("  {:?} {:?} {:?}\n", v.pos, v.colour, v.uv)),
            Vertices::Bloom(v) => v
                .iter()
                .for_each(|v| s += &format!("  {:?} {:?} {:?}\n", v.pos, v.colour, v.uv)),
        }
    }
    s
}

/// Every buffer non-empty; the users buffer has two avatar runs.
fn synthetic() -> Frame {
    let (beam, file, a, b) = (TextureId(1), TextureId(2), TextureId(3), TextureId(4));
    let col = Vec4::new(1.0, 0.5, 0.25, 0.75);
    let mut f = Frame {
        beam: Some(beam),
        file_graphic: Some(file),
        user_scale: 1.5,
        ..Frame::default()
    };
    f.edges.add(
        None,
        Vec2::new(10.0, 20.0),
        Vec2::new(30.0, 4.0),
        col,
        FULL_TEXCOORD,
    );
    f.actions.add(
        None,
        Vec2::new(-5.0, 5.0),
        Vec2::new(2.0, 3.0),
        col,
        FULL_TEXCOORD,
    );
    f.files.add(
        None,
        Vec2::new(0.0, 0.0),
        Vec2::new(8.0, 6.0),
        col,
        FULL_TEXCOORD,
    );
    f.users.add(
        Some(a),
        Vec2::new(1.0, 1.0),
        Vec2::new(9.0, 9.0),
        col,
        FULL_TEXCOORD,
    );
    f.users.add(
        Some(a),
        Vec2::new(2.0, 2.0),
        Vec2::new(9.0, 9.0),
        col,
        FULL_TEXCOORD,
    );
    f.users.add(
        Some(b),
        Vec2::new(3.0, 3.0),
        Vec2::new(9.0, 9.0),
        col,
        FULL_TEXCOORD,
    );
    f.bloom.add(
        Vec2::new(-4.0, -4.0),
        Vec2::new(8.0, 8.0),
        col,
        Vec4::new(4.0, 0.0, 0.0, 0.0),
    );
    f
}

#[test]
fn synthetic_frame_matches_snapshot() {
    let got = describe(&synthetic().batches());
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/snapshots/synthetic_frame.txt"
    );
    if matches!(std::env::var("BURL_BLESS").as_deref(), Ok("1")) {
        std::fs::write(path, &got).unwrap();
    }
    assert_eq!(got, std::fs::read_to_string(path).unwrap());
}

#[test]
fn passes_run_in_gource_order_and_empty_buffers_draw_nothing() {
    let passes: Vec<_> = synthetic().batches().iter().map(|b| b.pass).collect();
    use Pass::*;
    // Two user runs -> two shadow and two user batches.
    assert_eq!(
        passes,
        [
            EdgeShadows,
            Edges,
            FileShadows,
            Actions,
            Files,
            UserShadows,
            UserShadows,
            Users,
            Users,
            Bloom
        ]
    );
    assert!(Frame::default().batches().is_empty());
}

#[test]
fn quad_corners_follow_quadbuf_add() {
    let mut q = QuadBuf::default();
    q.add(
        None,
        Vec2::new(1.0, 2.0),
        Vec2::new(10.0, 20.0),
        Vec4::ONE,
        Vec4::new(0.1, 0.2, 0.3, 0.4),
    );
    let c: Vec<_> = q.vertices().iter().map(|v| (v.pos, v.uv)).collect();
    assert_eq!(
        c,
        [
            ([1.0, 2.0], [0.1, 0.2]),
            ([11.0, 2.0], [0.3, 0.2]),
            ([11.0, 22.0], [0.3, 0.4]),
            ([1.0, 22.0], [0.1, 0.4]),
        ]
    );
}

#[test]
fn indices_triangulate_each_quad() {
    assert_eq!(quad_indices(2), [0, 1, 2, 0, 2, 3, 4, 5, 6, 4, 6, 7]);
}

#[test]
fn flip_rows_reverses_row_order_not_pixels() {
    // 3 wide x 2 high, 1 byte per pixel: asymmetric in both axes.
    assert_eq!(flip_rows(&[1, 2, 3, 4, 5, 6], 3, 2, 1), [4, 5, 6, 1, 2, 3]);
    assert_eq!(flip_rows(&[1, 2, 3, 4], 1, 2, 2), [3, 4, 1, 2]);
}

#[test]
fn ortho_puts_pixel_origin_at_top_left() {
    let m = ortho_2d(320.0, 200.0);
    let p = m.project_point3(glam::Vec3::new(0.0, 0.0, 0.0));
    assert_eq!((p.x, p.y), (-1.0, 1.0));
    let p = m.project_point3(glam::Vec3::new(320.0, 200.0, 0.0));
    assert_eq!((p.x, p.y), (1.0, -1.0));
}

#[test]
fn background_is_centred_with_integer_halves_and_drawn_first() {
    let b = background_batch(
        TextureId(20),
        glam::Vec2::new(101.0, 51.0),
        glam::Vec2::new(640.0, 480.0),
    );
    assert_eq!(
        (b.pass, b.texture, b.space),
        (Pass::Background, Some(TextureId(20)), Space::Screen)
    );
    let Vertices::Quads(v) = &b.vertices else {
        panic!()
    };
    // 640/2 - 101/2 = 270, 480/2 - 51/2 = 215 (not 269.5 / 214.5).
    assert_eq!(v[0].pos, [270.0, 215.0]);
    assert_eq!(v[2].pos, [371.0, 266.0]);
    let f = Frame {
        underlay: vec![b],
        ..Frame::default()
    };
    assert_eq!(f.batches()[0].pass, Pass::Background);
}

#[test]
fn logo_sits_bottom_right_inside_its_offset() {
    let b = logo_batch(
        TextureId(21),
        glam::Vec2::new(64.0, 32.0),
        glam::Vec2::new(640.0, 480.0),
        glam::Vec2::new(10.0, 5.0),
    );
    let Vertices::Quads(v) = &b.vertices else {
        panic!()
    };
    assert_eq!(v[0].pos, [566.0, 443.0]);
    assert_eq!(v[2].pos, [630.0, 475.0]);
    assert_eq!(v[0].uv, [0.0, 0.0]);
    assert_eq!(b.pass, Pass::Overlay);
}
