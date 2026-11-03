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
    }
}

fn scene() -> SceneView {
    let file = FileView {
        pos: Vec2::new(10.0, 20.0),
        size: 8.4,
        colour: Vec3::new(1.0, 0.5, 0.0),
        alpha: 0.5,
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
            alpha: 1.0,
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
    }
}

fn frame() -> Frame {
    let tex = SceneTextures {
        beam: TextureId(1),
        file: TextureId(2),
        file_ratio: 1.0,
        user: TextureId(3),
        user_ratio: 512.0 / 384.0,
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
