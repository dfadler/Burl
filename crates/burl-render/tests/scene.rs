//! `build_frame` on a hand-built scene: snapshot of the batches (the GL-free regression layer).
//! `BURL_BLESS=1 cargo test -p burl-render --test scene` rewrites the snapshot.

use burl_render::*;
use burl_sim::view::*;
use glam::{Vec2, Vec3};

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

fn dir(parent: Option<usize>, pos: Vec2, visible: bool, files: Vec<FileView>) -> DirView {
    DirView {
        parent,
        pos,
        spos: Vec2::new(pos.y, -pos.x) * 0.5,
        colour: [0.5, 0.25, 1.0, 1.0],
        radius: 6.0,
        visible,
        files,
        name: "d".into(),
        depth: 1,
        since_last_node_change: 0.0,
    }
}

fn scene() -> SceneView {
    let file = FileView {
        pos: Vec2::new(10.0, 20.0),
        size: 8.4,
        colour: Vec3::new(1.0, 0.5, 0.0),
        alpha: 0.5,
        name: "f.rs".into(),
        nametime: 4.0,
        name_interval: 4.0,
        selected: false,
    };
    SceneView {
        camera: CameraView {
            pos: Vec3::new(0.0, 0.0, -300.0),
            target: Vec3::ZERO,
            max_distance: 10000.0,
        },
        dirs: vec![
            dir(None, Vec2::ZERO, true, vec![]),
            dir(Some(0), Vec2::new(30.0, 10.0), true, vec![file]),
            // An invisible dir draws no edge and no bloom.
            dir(Some(0), Vec2::new(-40.0, 5.0), false, vec![]),
        ],
        users: vec![UserView {
            pos: Vec2::new(5.0, -5.0),
            size: 20.0,
            colour: Vec3::ONE,
            graphic_ratio: 512.0 / 384.0,
            graphic: None,
            alpha: 1.0,
            name: "u".into(),
            nametime: 5.0,
            name_interval: 5.0,
            highlighted: false,
            selected: false,
            actions: vec![
                ActionView {
                    src: Vec2::new(5.0, -5.0),
                    dest: Vec2::new(40.0, 30.0),
                    target_size: 8.4,
                    progress: 0.25,
                    colour: Vec3::new(0.0, 1.0, 0.0),
                },
                // Finished actions are skipped.
                ActionView {
                    src: Vec2::ZERO,
                    dest: Vec2::ONE,
                    target_size: 8.4,
                    progress: 1.0,
                    colour: Vec3::ONE,
                },
            ],
        }],
        time: 0,
        seekable: false,
        captions: vec![],
        captions_activated: 0,
        key_events: vec![],
    }
}

fn frame() -> Frame {
    let tex = SceneTextures {
        beam: TextureId(1),
        file: TextureId(2),
        file_ratio: 1.0,
        user: TextureId(3),
    };
    build_frame(&scene(), &tex, &SceneSettings::new(640, 480))
}

#[test]
fn scene_frame_matches_snapshot() {
    let got = describe(&frame().batches());
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/snapshots/scene_frame.txt"
    );
    if matches!(std::env::var("BURL_BLESS").as_deref(), Ok("1")) {
        std::fs::write(path, &got).unwrap();
    }
    assert_eq!(got, std::fs::read_to_string(path).unwrap());
}

#[test]
fn only_visible_dirs_get_edges_and_bloom_and_finished_actions_are_skipped() {
    let f = frame();
    // One visible child -> one spline of 1..=10 quads (4 vertices each).
    let n = f.edges.vertices().len();
    assert!((4..=40).contains(&n) && n.is_multiple_of(4));
    assert_eq!(f.bloom.vertices.len(), 2 * 4);
    assert_eq!(f.actions.vertices().len(), 4);
    assert_eq!(f.files.vertices().len(), 4);
    assert_eq!(f.users.vertices().len(), 4);
}

#[test]
fn hide_flags_empty_their_buffers() {
    let tex = SceneTextures {
        beam: TextureId(1),
        file: TextureId(2),
        file_ratio: 1.0,
        user: TextureId(3),
    };
    let go = |f: fn(&mut SceneSettings)| {
        let mut s = SceneSettings::new(640, 480);
        f(&mut s);
        build_frame(&scene(), &tex, &s)
    };
    assert!(go(|s| s.hide_bloom = true).bloom.vertices.is_empty());
    assert!(go(|s| s.hide_tree = true).edges.vertices().is_empty());
    assert!(go(|s| s.hide_files = true).files.vertices().is_empty());
    let f = go(|s| s.hide_users = true);
    assert!(f.users.vertices().is_empty() && f.actions.vertices().is_empty());
    // Each flag leaves the other buffers alone.
    assert!(!go(|s| s.hide_users = true).files.vertices().is_empty());
}

#[test]
fn an_avatar_user_draws_with_its_own_texture_and_ratio() {
    let mut sc = scene();
    sc.users[0].graphic = Some(900);
    sc.users[0].graphic_ratio = 2.0;
    let tex = SceneTextures {
        beam: TextureId(1),
        file: TextureId(2),
        file_ratio: 1.0,
        user: TextureId(3),
    };
    let f = build_frame(&sc, &tex, &SceneSettings::new(640, 480));
    let v = f.users.vertices();
    assert_eq!(v[2].pos[1] - v[0].pos[1], 2.0 * (v[1].pos[0] - v[0].pos[0]));
    assert!(f
        .batches()
        .iter()
        .any(|b| b.texture == Some(TextureId(900))));
}

#[test]
fn fixed_user_size_scales_users_with_camera_distance() {
    let tex = SceneTextures {
        beam: TextureId(1),
        file: TextureId(2),
        file_ratio: 1.0,
        user: TextureId(3),
    };
    let mut sc = scene();
    sc.camera.pos.z = -600.0;
    let mut ss = SceneSettings::new(640, 480);
    let width = |f: &Frame| {
        let v = f.users.vertices();
        v[2].pos[0] - v[0].pos[0]
    };
    let plain = width(&build_frame(&sc, &tex, &ss));
    ss.fixed_user_size = true;
    let fixed = width(&build_frame(&sc, &tex, &ss));
    assert!((fixed - 2.0 * plain).abs() < 1e-3, "{plain} {fixed}");
}

#[test]
fn hide_root_skips_edges_to_the_roots_children() {
    let tex = SceneTextures {
        beam: TextureId(1),
        file: TextureId(2),
        file_ratio: 1.0,
        user: TextureId(3),
    };
    let mut sc = scene();
    // a grandchild keeps its edge under hide_root
    sc.dirs
        .push(dir(Some(1), Vec2::new(60.0, 20.0), true, vec![]));
    let mut ss = SceneSettings::new(640, 480);
    let both = build_frame(&sc, &tex, &ss).edges.vertices().len();
    ss.hide_root = true;
    let one = build_frame(&sc, &tex, &ss).edges.vertices().len();
    assert!(one > 0 && one < both);
    sc.dirs.pop();
    assert_eq!(build_frame(&sc, &tex, &ss).edges.vertices().len(), 0);
}

#[test]
fn selected_sprites_are_white() {
    let mut sc = scene();
    sc.dirs[1].files[0].selected = true;
    sc.users[0].selected = true;
    let tex = SceneTextures {
        beam: TextureId(1),
        file: TextureId(2),
        file_ratio: 1.0,
        user: TextureId(3),
    };
    let f = build_frame(&sc, &tex, &SceneSettings::new(640, 480));
    assert!(f
        .files
        .vertices()
        .iter()
        .all(|v| v.colour[..3] == [1.0, 1.0, 1.0]));
    assert!(f
        .users
        .vertices()
        .iter()
        .all(|v| v.colour[..3] == [1.0, 1.0, 1.0]));
}

/// G7: `RDirNode::checkFrustum`. Gource draws no files or bloom for a dir whose bounds
/// (`pos +- radius`) miss the frustum, even when its bloom (twice the radius) reaches into view.
/// At z = -300, fov 90 and 640x480 the view's half width at z = 0 is 400.
#[test]
fn deviation_g7_culled_dir_draws_no_files_or_bloom() {
    let tex = SceneTextures {
        beam: TextureId(1),
        file: TextureId(2),
        file_ratio: 1.0,
        user: TextureId(3),
    };
    let count = |x: f32| {
        let mut sc = scene();
        sc.dirs[1].pos = Vec2::new(x, 0.0);
        sc.dirs[1].files[0].pos = Vec2::new(x, 0.0);
        let f = build_frame(&sc, &tex, &SceneSettings::new(640, 480));
        (f.bloom.vertices.len() / 4, f.files.vertices().len() / 4)
    };
    assert_eq!(count(300.0), (2, 1), "in view");
    // Bounds start at x = 404 (off screen) but the bloom quad starts at 398 (on screen).
    assert_eq!(
        count(410.0),
        (1, 0),
        "culled: bloom reaching in is not drawn"
    );
    // The camera moving makes the same dir visible again.
    let mut sc = scene();
    sc.dirs[1].pos = Vec2::new(410.0, 0.0);
    sc.camera.pos.x = 200.0;
    sc.camera.target.x = 200.0;
    let f = build_frame(&sc, &tex, &SceneSettings::new(640, 480));
    assert_eq!(f.bloom.vertices.len() / 4, 2);
}
