//! `SceneView` -> `Frame`: the CPU half of `Gource::draw` (phase 5, ticket #19). Ported from
//! `Gource::updateVBOs` (src/gource.cpp:2256), `RDirNode::updateFilesVBO/updateBloomVBO/
//! updateEdgeVBO/calcEdges/calcScreenPos` (src/dirnode.cpp), `SplineEdge::update/drawToVBO`
//! (src/spline.cpp) and `RAction::drawToVBO` (src/action.cpp).
//!
//! Not ported: the `in_frustum` culling of files and bloom (it only skips off-screen quads, so the
//! image is unchanged), `fixed_user_size`, selection.

use crate::{BloomBuf, Camera, Frame, QuadBuf, QuadVertex, TextureId, FULL_TEXCOORD};
use burl_sim::view::{DirView, SceneView};
use burl_util::math::normalise_vec2;
use glam::{Vec2, Vec3, Vec4};

/// The textures a frame uses and their height/width ratios (`Pawn::graphic_ratio`).
#[derive(Clone, Copy, Debug)]
pub struct SceneTextures {
    pub beam: TextureId,
    pub file: TextureId,
    pub file_ratio: f32,
    pub user: TextureId,
    pub user_ratio: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct SceneSettings {
    pub width: f32,
    pub height: f32,
    /// `--bloom-multiplier` (default 1.0) and `--bloom-intensity` (default 0.75).
    pub bloom_multiplier: f32,
    pub bloom_intensity: f32,
    pub hide_bloom: bool,
    /// `--hide tree/files/users`: the edges, the files (and their shadows), the users and actions.
    pub hide_tree: bool,
    pub hide_files: bool,
    pub hide_users: bool,
    /// `--user-scale`, which also scales the user shadow offset.
    pub user_scale: f32,
}

impl SceneSettings {
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width: width as f32,
            height: height as f32,
            bloom_multiplier: 1.0,
            bloom_intensity: 0.75,
            hide_bloom: false,
            hide_tree: false,
            hide_files: false,
            hide_users: false,
            user_scale: 1.0,
        }
    }
}

/// `SplineEdge::update` + `drawToVBO` for one edge, appended to `buf`. Positions are screen pixels.
fn spline_edge(buf: &mut QuadBuf, pos1: Vec2, col1: Vec4, pos2: Vec2, col2: Vec4, spos: Vec2) {
    let mid = (pos1 - pos2) * 0.5;
    let to = pos1 - spos;
    let dp = f32::min(1.0, normalise_vec2(to).dot(normalise_vec2(mid)));
    let ang = dp.acos() / std::f32::consts::PI;
    let detail = ((ang * 100.0) as i32).clamp(1, 10);

    let points: Vec<(Vec2, Vec4)> = (0..=detail)
        .map(|i| {
            let t = i as f32 / detail as f32;
            let tt = 1.0 - t;
            let p0 = pos1 * t + spos * tt;
            let p1 = spos * t + pos2 * tt;
            (p0 * t + p1 * tt, col1 * t + col2 * tt)
        })
        .collect();

    for w in points.windows(2) {
        let ((a, ca), (b, cb)) = (w[0], w[1]);
        let d = a - b;
        let perp = normalise_vec2(Vec2::new(-d.y, d.x)) * 2.5;
        let v = |p: Vec2, c: Vec4, u: f32| QuadVertex {
            pos: p.to_array(),
            colour: c.to_array(),
            uv: [u, 0.0],
        };
        buf.add_vertices(
            None,
            [
                v(a + perp, ca, 1.0),
                v(a - perp, ca, 0.0),
                v(b - perp, cb, 0.0),
                v(b + perp, cb, 1.0),
            ],
        );
    }
}

/// The camera's projection * view for the scene's world-space batches.
pub fn world_matrix(scene: &SceneView, s: &SceneSettings) -> glam::Mat4 {
    camera(scene).view_projection(s.width, s.height)
}

fn camera(scene: &SceneView) -> Camera {
    Camera {
        pos: scene.camera.pos,
        target: scene.camera.target,
        max_distance: scene.camera.max_distance,
    }
}

/// `Gource::updateVBOs` plus the edge/bloom buffers: everything `drawScene` draws.
pub fn build_frame(scene: &SceneView, tex: &SceneTextures, s: &SceneSettings) -> Frame {
    let cam = camera(scene);
    let mut f = Frame {
        beam: Some(tex.beam),
        file_graphic: Some(tex.file),
        user_scale: s.user_scale,
        ..Frame::default()
    };

    // calcScreenPos: dir and spline-point positions on screen (z = 0 plane).
    let screen = |p: Vec2| {
        cam.project(Vec3::new(p.x, p.y, 0.0), s.width, s.height)
            .truncate()
    };
    let proj: Vec<(Vec2, Vec2)> = scene
        .dirs
        .iter()
        .map(|d| (screen(d.pos), screen(d.spos)))
        .collect();
    let col = |d: &DirView| Vec4::from_array(d.colour);

    // Edges: every visible dir with a parent (an invisible dir has no visible descendants).
    for (i, d) in scene.dirs.iter().enumerate().filter(|_| !s.hide_tree) {
        if let (Some(p), true) = (d.parent, d.visible) {
            let (pp, _) = proj[p];
            let (dp, ds) = proj[i];
            spline_edge(&mut f.edges, pp, col(&scene.dirs[p]), dp, col(d), ds);
        }
    }

    for d in scene.dirs.iter().filter(|_| !s.hide_files) {
        for file in &d.files {
            let dims = Vec2::new(file.size, file.size * tex.file_ratio);
            f.files.add(
                None,
                file.pos - dims * 0.5,
                dims,
                file.colour.extend(file.alpha),
                FULL_TEXCOORD,
            );
        }
    }

    for u in scene.users.iter().filter(|_| !s.hide_users) {
        let dims = Vec2::new(u.size, u.size * tex.user_ratio);
        f.users.add(
            Some(tex.user),
            u.pos - dims * 0.5,
            dims,
            u.colour.extend(u.alpha),
            FULL_TEXCOORD,
        );
        for a in u.actions.iter().filter(|a| a.progress < 1.0) {
            let n = normalise_vec2(a.dest - a.src);
            let offset = Vec2::new(-n.y, n.x) * a.target_size * 0.5;
            let offset_src = offset * 0.3;
            let (alpha, alpha2) = (1.0 - a.progress, (1.0 - a.progress) * 0.1);
            let (c1, c2) = (a.colour.extend(alpha), a.colour.extend(alpha2));
            let v = |p: Vec2, c: Vec4, uv: [f32; 2]| QuadVertex {
                pos: p.to_array(),
                colour: c.to_array(),
                uv,
            };
            f.actions.add_vertices(
                None,
                [
                    v(a.src - offset_src, c2, [0.0, 0.0]),
                    v(a.src + offset_src, c2, [0.0, 1.0]),
                    v(a.dest + offset, c1, [1.0, 1.0]),
                    v(a.dest - offset, c1, [1.0, 0.0]),
                ],
            );
        }
    }

    if !s.hide_bloom {
        f.bloom = BloomBuf::default();
        for d in scene.dirs.iter().filter(|d| d.visible) {
            let radius = d.radius * 2.0 * s.bloom_multiplier;
            let dims = Vec2::splat(radius * 2.0);
            let c = col(d) * s.bloom_intensity;
            f.bloom.add(
                d.pos - dims * 0.5,
                dims,
                Vec4::new(c.x, c.y, c.z, 1.0),
                Vec4::new(radius, d.pos.x, d.pos.y, 0.0),
            );
        }
    }
    f
}
