//! Name labels on a hand-built scene: snapshot of the `Pass::Names` batches plus the visibility and
//! fade rules (no GL). `BURL_BLESS=1 cargo test -p burl-render --test names` rewrites the snapshot.

use burl_render::*;
use burl_sim::view::*;
use glam::{Vec2, Vec3};

fn dir(parent: Option<usize>, name: &str, depth: i32, age: f32, files: Vec<FileView>) -> DirView {
    let pos = if parent.is_some() {
        Vec2::new(30.0, 10.0)
    } else {
        Vec2::ZERO
    };
    DirView {
        parent,
        pos,
        spos: Vec2::new(pos.y, -pos.x) * 0.5,
        colour: [0.5, 0.25, 1.0, 1.0],
        radius: 6.0,
        visible: true,
        files,
        name: name.into(),
        depth,
        since_last_node_change: age,
    }
}

fn file(name: &str, interval: f32) -> FileView {
    FileView {
        pos: Vec2::new(10.0, 20.0),
        size: 8.4,
        colour: Vec3::ONE,
        alpha: 1.0,
        name: name.into(),
        nametime: 4.0,
        name_interval: interval,
    }
}

fn user(name: &str, interval: f32, highlighted: bool) -> UserView {
    UserView {
        pos: Vec2::new(5.0, -5.0),
        size: 20.0,
        colour: Vec3::ONE,
        alpha: 0.75,
        actions: vec![],
        name: name.into(),
        nametime: 5.0,
        name_interval: interval,
        highlighted,
    }
}

fn scene(dirs: Vec<DirView>, users: Vec<UserView>) -> SceneView {
    SceneView {
        camera: CameraView {
            pos: Vec3::new(0.0, 0.0, -300.0),
            target: Vec3::ZERO,
            max_distance: 10000.0,
        },
        dirs,
        users,
    }
}

fn names(scene: &SceneView, tweak: impl FnOnce(&mut NameSettings, &mut SceneSettings)) -> Frame {
    let tex = SceneTextures {
        beam: TextureId(1),
        file: TextureId(2),
        file_ratio: 1.0,
        user: TextureId(3),
        user_ratio: 512.0 / 384.0,
    };
    let (mut n, mut s) = (NameSettings::default(), SceneSettings::new(640, 480));
    tweak(&mut n, &mut s);
    let mut mgr = FontManager::new(FREE_SANS, 100).unwrap();
    let mut fonts = NameFonts::new(&mut mgr, 11, 10, 12);
    mgr.start_buffer();
    draw_names(&mut mgr, &mut fonts, scene, &tex, &s, &n);
    Frame {
        names: std::mem::take(&mut mgr.buffer),
        ..Frame::default()
    }
}

fn quads(f: &Frame) -> usize {
    f.names.vertices().len() / 4
}

fn full_scene() -> SceneView {
    scene(
        vec![
            dir(None, "", 0, 0.0, vec![]),
            // Two seconds old: alpha (5 - 2) / 5.
            dir(Some(0), "src", 1, 2.0, vec![file("main.rs", 2.0)]),
        ],
        vec![user("ann", 2.5, false)],
    )
}

#[test]
fn names_match_snapshot() {
    let f = names(&full_scene(), |_, _| {});
    let mut got = String::new();
    for b in f.batches() {
        got += &format!(
            "{:?} {:?} tex={:?}\n",
            b.pass,
            b.pipeline,
            b.texture.map(|t| t.0)
        );
        if let Vertices::Quads(v) = &b.vertices {
            for v in v {
                got += &format!("  {:?} {:?} {:?}\n", v.pos, v.colour, v.uv);
            }
        }
    }
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/snapshots/names_frame.txt"
    );
    if matches!(std::env::var("BURL_BLESS").as_deref(), Ok("1")) {
        std::fs::write(path, &got).unwrap();
    }
    assert_eq!(got, std::fs::read_to_string(path).unwrap());
}

#[test]
fn every_label_is_drawn_once_by_default() {
    // "src" + "main.rs" + "ann": 3 + 7 + 3 glyph quads, none of them blank.
    assert_eq!(quads(&names(&full_scene(), |_, _| {})), 13);
}

#[test]
fn hide_flags_drop_their_labels() {
    let s = full_scene();
    assert_eq!(quads(&names(&s, |n, _| n.hide_dirnames = true)), 10);
    assert_eq!(quads(&names(&s, |n, _| n.hide_filenames = true)), 6);
    assert_eq!(quads(&names(&s, |_, s| s.hide_files = true)), 6);
    assert_eq!(quads(&names(&s, |n, _| n.hide_usernames = true)), 10);
    assert_eq!(quads(&names(&s, |_, s| s.hide_users = true)), 10);
}

#[test]
fn dir_names_follow_age_depth_and_highlight() {
    let aged = |age| {
        scene(
            vec![
                dir(None, "", 0, 0.0, vec![]),
                dir(Some(0), "src", 1, age, vec![]),
            ],
            vec![],
        )
    };
    assert_eq!(quads(&names(&aged(4.9), |_, _| {})), 3);
    assert_eq!(quads(&names(&aged(5.1), |_, _| {})), 0);
    assert_eq!(quads(&names(&aged(5.1), |n, _| n.highlight_dirs = true)), 3);
    // --dir-name-depth 1 hides directories deeper than 2 (depth - 1 > 1).
    let deep = scene(
        vec![
            dir(None, "", 0, 0.0, vec![]),
            dir(Some(0), "src", 3, 0.0, vec![]),
        ],
        vec![],
    );
    assert_eq!(quads(&names(&deep, |n, _| n.dir_name_depth = 1)), 0);
    assert_eq!(quads(&names(&deep, |_, _| {})), 3);
    let edge = scene(
        vec![
            dir(None, "", 0, 0.0, vec![]),
            dir(Some(0), "src", 2, 0.0, vec![]),
        ],
        vec![],
    );
    assert_eq!(quads(&names(&edge, |n, _| n.dir_name_depth = 1)), 3);
    // The root has no name.
    let root = scene(vec![dir(None, "x", 0, 0.0, vec![])], vec![]);
    assert_eq!(quads(&names(&root, |_, _| {})), 0);
}

#[test]
fn faded_names_and_extension_labels() {
    let one = |interval| {
        scene(
            vec![dir(None, "", 0, 9.0, vec![file("a.rs", interval)])],
            vec![],
        )
    };
    assert_eq!(quads(&names(&one(2.0), |_, _| {})), 4);
    // Expired (interval < 0), or alpha <= 0.01 right after being shown / at the end of the fade.
    assert_eq!(quads(&names(&one(-0.1), |_, _| {})), 0);
    assert_eq!(quads(&names(&one(4.0), |_, _| {})), 0);
    assert_eq!(quads(&names(&one(0.005), |_, _| {})), 0);
    assert_eq!(quads(&names(&one(2.0), |n, _| n.file_extensions = true)), 2);
}

#[test]
fn highlighted_users_keep_their_name_after_the_fade() {
    let u = |highlighted| {
        scene(
            vec![dir(None, "", 0, 9.0, vec![])],
            vec![user("ann", -1.0, highlighted)],
        )
    };
    assert_eq!(quads(&names(&u(false), |_, _| {})), 0);
    let f = names(&u(true), |n, _| {
        n.highlight_colour = Vec3::new(1.0, 0.0, 0.0)
    });
    assert_eq!(quads(&f), 3);
    // Highlighted names use the highlight colour and the user's own alpha.
    assert_eq!(f.names.vertices()[0].colour, [1.0, 0.0, 0.0, 0.75]);
    assert_eq!(
        quads(&names(&u(false), |n, _| n.highlight_all_users = true)),
        3
    );
}

#[test]
fn name_alpha_ports_pawn_draw_name() {
    // nametime 5: fade in over the first second, hold, fade out over the last.
    let a = |interval| name_alpha(5.0, interval);
    assert_eq!(a(-0.5), None);
    assert_eq!(a(5.0), Some(0.0));
    assert_eq!(a(4.5), Some(0.5));
    assert_eq!(a(2.5), Some(1.0));
    assert_eq!(a(0.5), Some(0.5));
    assert_eq!(a(0.0), Some(0.0));
    // done == 1.0 falls through to the fade-out branch, as in C++.
    assert_eq!(a(4.0), Some(4.0));
}

#[test]
fn dir_label_sits_on_the_edge() {
    // Default position 0.5 is the spline control point's half weight: (p1 + p2) / 4 + spos / 2.
    let (p1, p2, sp) = (
        Vec2::new(0.0, 0.0),
        Vec2::new(8.0, 4.0),
        Vec2::new(2.0, 6.0),
    );
    let got = label_pos(p1, p2, sp, 0.5);
    assert!((got - (p1 * 0.25 + p2 * 0.25 + sp * 0.5)).length() < 1e-6);
    assert_eq!(label_pos(p1, p2, sp, 1.0), p2 * 1.0);
}
