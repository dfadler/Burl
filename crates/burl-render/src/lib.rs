#![forbid(unsafe_code)]
//! CPU-side render batches (phase 5, ticket #19). Builds the draw batches in Gource's fixed pass
//! order from already-computed vertices; no GL here, so the order, pipelines and vertices are
//! snapshot-tested without a context. `burl-gl` draws the batches.
//!
//! Ported from `Gource::drawScene` (src/gource.cpp:2062-2172, 2196-2400) and `quadbuf`/`bloombuf`
//! (reference/gource-cpp-dump/src/core/vbo.cpp, src/bloom.cpp). Row numbers below are the pass
//! table in docs/rust-rewrite/discovery/output-pipeline.md.

mod camera;
mod font;
mod labels;
mod scene;
pub use camera::*;
pub use font::*;
pub use labels::*;
pub use scene::*;

pub use glam;
use glam::{Mat4, Vec2, Vec4};

/// `quadbuf_vertex`: pos, colour, uv (32 bytes).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct QuadVertex {
    pub pos: [f32; 2],
    pub colour: [f32; 4],
    pub uv: [f32; 2],
}

/// `bloom_vertex`: pos, colour, and a vec4 texcoord that carries the radius scale in `x` and the
/// node centre in `yz` (bloom.vert). The C++ struct pads this to 64 bytes; the pad is not kept.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct BloomVertex {
    pub pos: [f32; 2],
    pub colour: [f32; 4],
    pub uv: [f32; 4],
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct TextureId(pub u32);

/// Texture coordinates `(u0, v0, u1, v1)` covering the whole image (`quadbuf_default_texcoord`).
pub const FULL_TEXCOORD: Vec4 = Vec4::new(0.0, 0.0, 1.0, 1.0);

/// Port of `quadbuf`: four vertices per quad plus the per-texture runs (`quadbuf_tex`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct QuadBuf {
    vertices: Vec<QuadVertex>,
    /// `(first vertex, texture)`; a run lasts until the next run starts.
    runs: Vec<(usize, TextureId)>,
}

impl QuadBuf {
    pub fn vertices(&self) -> &[QuadVertex] {
        &self.vertices
    }

    /// `quadbuf::reset`. Both fields go together so a run can never point past the vertices.
    pub fn clear(&mut self) {
        self.vertices.clear();
        self.runs.clear();
    }

    /// `quadbuf::add(textureid, v1, v2, v3, v4)`: four ready-made vertices (splines, actions).
    pub fn add_vertices(&mut self, texture: Option<TextureId>, quad: [QuadVertex; 4]) {
        let i = self.vertices.len();
        self.vertices.extend(quad);
        if let Some(t) = texture {
            if self.runs.last().map(|r| r.1) != Some(t) {
                self.runs.push((i, t));
            }
        }
    }

    /// `quadbuf::add(textureid, pos, dims, colour, texcoord)`. A zero texture id starts no run.
    pub fn add(
        &mut self,
        texture: Option<TextureId>,
        pos: Vec2,
        dims: Vec2,
        colour: Vec4,
        texcoord: Vec4,
    ) {
        let v = |p: Vec2, u: f32, w: f32| QuadVertex {
            pos: p.to_array(),
            colour: colour.to_array(),
            uv: [u, w],
        };
        let i = self.vertices.len();
        self.vertices.extend([
            v(pos, texcoord.x, texcoord.y),
            v(pos + Vec2::new(dims.x, 0.0), texcoord.z, texcoord.y),
            v(pos + dims, texcoord.z, texcoord.w),
            v(pos + Vec2::new(0.0, dims.y), texcoord.x, texcoord.w),
        ]);
        if let Some(t) = texture {
            if self.runs.last().map(|r| r.1) != Some(t) {
                self.runs.push((i, t));
            }
        }
    }

    /// The `(texture, vertices)` ranges `quadbuf::draw` issues. Without runs the whole buffer is
    /// one untextured range; with runs, anything before the first run is not drawn (as in C++).
    pub(crate) fn ranges(&self) -> Vec<(Option<TextureId>, &[QuadVertex])> {
        if self.vertices.is_empty() {
            return vec![];
        }
        if self.runs.is_empty() {
            return vec![(None, &self.vertices[..])];
        }
        let ends = self
            .runs
            .iter()
            .skip(1)
            .map(|r| r.0)
            .chain([self.vertices.len()]);
        self.runs
            .iter()
            .zip(ends)
            .map(|(&(start, t), end)| (Some(t), &self.vertices[start..end]))
            .collect()
    }
}

/// Port of `bloombuf`: bloom quads share one texture-less pipeline.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BloomBuf {
    pub vertices: Vec<BloomVertex>,
}

impl BloomBuf {
    /// `bloombuf::add`.
    pub fn add(&mut self, pos: Vec2, dims: Vec2, colour: Vec4, texcoord: Vec4) {
        let v = |p: Vec2| BloomVertex {
            pos: p.to_array(),
            colour: colour.to_array(),
            uv: texcoord.to_array(),
        };
        self.vertices.extend([
            v(pos),
            v(pos + Vec2::new(dims.x, 0.0)),
            v(pos + dims),
            v(pos + Vec2::new(0.0, dims.y)),
        ]);
    }
}

/// The passes of `Gource::drawScene`, in draw order (output-pipeline.md rows 2-8).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Pass {
    /// The `--background-image`, drawn right after the clear (`Gource::drawBackground`).
    Background,
    EdgeShadows,
    Edges,
    FileShadows,
    Actions,
    Files,
    UserShadows,
    Users,
    Bloom,
    /// Dir and user names: `fontmanager.drawBuffer()` under the text shader (gource.cpp:2525).
    Names,
    /// Everything drawn after the names in 2D: logo, splash, date, title, captions, key, text boxes.
    Overlay,
}

/// Which shader a batch uses.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Pipeline {
    /// The C++ draws these with program 0: fixed-function `texture * vertex colour`.
    Plain,
    /// shadow.vert/frag: black, alpha `colour.a * texture.a * shadow_strength`.
    Shadow,
    /// bloom.vert/frag.
    Bloom,
    /// text.vert/frag: coverage in alpha, drop shadow from the texel up and left of the sample.
    Text,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Blend {
    /// `glBlendFunc(GL_SRC_ALPHA, GL_ONE_MINUS_SRC_ALPHA)`.
    Alpha,
    /// `glBlendFunc(GL_ONE, GL_ONE)`.
    Additive,
}

/// Edges are drawn under a temporary `glOrtho(0, w, h, 0, -1, 1)` (screen pixels); every other
/// pass uses the camera's projection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Space {
    Screen,
    World,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Vertices {
    Quads(Vec<QuadVertex>),
    Bloom(Vec<BloomVertex>),
}

/// One draw call: `vertices` are quads (4 per quad), triangulated by the drawer.
#[derive(Clone, Debug, PartialEq)]
pub struct Batch {
    pub pass: Pass,
    pub pipeline: Pipeline,
    pub blend: Blend,
    pub space: Space,
    /// Added to every vertex position (the C++ `glTranslatef` for shadows).
    pub offset: [f32; 2],
    pub texture: Option<TextureId>,
    pub vertices: Vertices,
}

/// `gGourceShadowStrength`: the `shadow_strength` uniform of every shadow pass.
pub const SHADOW_STRENGTH: f32 = 0.5;

/// The per-pass buffers of one frame, as `Gource::updateVBOs` leaves them.
#[derive(Clone, Debug)]
pub struct Frame {
    pub edges: QuadBuf,
    pub actions: QuadBuf,
    pub files: QuadBuf,
    pub users: QuadBuf,
    pub bloom: BloomBuf,
    /// `fontmanager.font_vbo`: dir and user names, drawn after the scene under the text shader.
    pub names: QuadBuf,
    /// Pre-built batches drawn first, in order (the background image).
    pub underlay: Vec<Batch>,
    /// Pre-built batches drawn last, in order (the 2D overlays).
    pub overlay: Vec<Batch>,
    /// The beam image `edges` and `actions` are textured with.
    pub beam: Option<TextureId>,
    /// The shared file image `files` is textured with.
    pub file_graphic: Option<TextureId>,
    /// `gGourceSettings.user_scale`: scales the user shadow offset.
    pub user_scale: f32,
}

impl Frame {
    /// The batches in draw order. Empty buffers produce no batch (the C++ returns early).
    pub fn batches(&self) -> Vec<Batch> {
        let mut out = self.underlay.clone();
        let mut quads =
            |pass, pipeline, space, offset: [f32; 2], buf: &QuadBuf, tex: Option<TextureId>| {
                for (t, v) in buf.ranges() {
                    out.push(Batch {
                        pass,
                        pipeline,
                        blend: Blend::Alpha,
                        space,
                        offset,
                        // A run's own texture wins; otherwise the pass's shared one.
                        texture: t.or(tex),
                        vertices: Vertices::Quads(v.to_vec()),
                    });
                }
            };
        let (shadow, user_shadow) = ([2.0, 2.0], [2.0 * self.user_scale, 2.0 * self.user_scale]);
        quads(
            Pass::EdgeShadows,
            Pipeline::Shadow,
            Space::Screen,
            shadow,
            &self.edges,
            self.beam,
        );
        quads(
            Pass::Edges,
            Pipeline::Plain,
            Space::Screen,
            [0.0; 2],
            &self.edges,
            self.beam,
        );
        quads(
            Pass::FileShadows,
            Pipeline::Shadow,
            Space::World,
            shadow,
            &self.files,
            self.file_graphic,
        );
        quads(
            Pass::Actions,
            Pipeline::Plain,
            Space::World,
            [0.0; 2],
            &self.actions,
            self.beam,
        );
        quads(
            Pass::Files,
            Pipeline::Plain,
            Space::World,
            [0.0; 2],
            &self.files,
            self.file_graphic,
        );
        quads(
            Pass::UserShadows,
            Pipeline::Shadow,
            Space::World,
            user_shadow,
            &self.users,
            None,
        );
        quads(
            Pass::Users,
            Pipeline::Plain,
            Space::World,
            [0.0; 2],
            &self.users,
            None,
        );
        if !self.bloom.vertices.is_empty() {
            out.push(Batch {
                pass: Pass::Bloom,
                pipeline: Pipeline::Bloom,
                blend: Blend::Additive,
                space: Space::World,
                offset: [0.0; 2],
                texture: None,
                vertices: Vertices::Bloom(self.bloom.vertices.clone()),
            });
        }
        out.extend(quad_batches(
            Pass::Names,
            Pipeline::Text,
            Blend::Alpha,
            Space::Screen,
            [0.0; 2],
            &self.names,
            None,
        ));
        out.extend(self.overlay.iter().cloned());
        out
    }
}

/// The batches `quadbuf::draw` issues for `buf`: one per texture run.
pub(crate) fn quad_batches(
    pass: Pass,
    pipeline: Pipeline,
    blend: Blend,
    space: Space,
    offset: [f32; 2],
    buf: &QuadBuf,
    texture: Option<TextureId>,
) -> Vec<Batch> {
    buf.ranges()
        .into_iter()
        .map(|(t, v)| Batch {
            pass,
            pipeline,
            blend,
            space,
            offset,
            // A run's own texture wins; otherwise the pass's shared one.
            texture: t.or(texture),
            vertices: Vertices::Quads(v.to_vec()),
        })
        .collect()
}

impl Default for Frame {
    fn default() -> Self {
        Self {
            edges: QuadBuf::default(),
            actions: QuadBuf::default(),
            files: QuadBuf::default(),
            users: QuadBuf::default(),
            bloom: BloomBuf::default(),
            names: QuadBuf::default(),
            underlay: Vec::new(),
            overlay: Vec::new(),
            beam: None,
            file_graphic: None,
            // Settings default (and reject <= 0), so an omitted scale must not zero the shadow offset.
            user_scale: 1.0,
        }
    }
}

/// One white, untinted image quad in screen space (the background and logo draws).
fn image_batch(pass: Pass, texture: TextureId, pos: Vec2, dims: Vec2) -> Batch {
    let mut buf = QuadBuf::default();
    buf.add(Some(texture), pos, dims, Vec4::ONE, FULL_TEXCOORD);
    quad_batches(
        pass,
        Pipeline::Plain,
        Blend::Alpha,
        Space::Screen,
        [0.0; 2],
        &buf,
        None,
    )
    .remove(0)
}

/// `Gource::drawBackground`: the image at its own size, centred (integer halves, as in C++).
pub fn background_batch(texture: TextureId, image: Vec2, screen: Vec2) -> Batch {
    let half = |s: f32, i: f32| ((s as i32) / 2 - (i as i32) / 2) as f32;
    let pos = Vec2::new(half(screen.x, image.x), half(screen.y, image.y));
    image_batch(Pass::Background, texture, pos, image)
}

/// The logo block of `Gource::draw`: the image at its own size, bottom right, `--logo-offset` in.
pub fn logo_batch(texture: TextureId, image: Vec2, screen: Vec2, offset: Vec2) -> Batch {
    image_batch(Pass::Overlay, texture, screen - image - offset, image)
}

/// `glOrtho(0, w, h, 0, -1, 1)` (`Display::mode2D`): pixels, y down.
pub fn ortho_2d(width: f32, height: f32) -> Mat4 {
    // glOrtho(l, r, b, t, n, f) with l=0, r=w, b=h, t=0, n=-1, f=1: x' = 2x/w - 1, y' = 1 - 2y/h, z' = -z.
    Mat4::from_cols_array(&[
        2.0 / width,
        0.0,
        0.0,
        0.0, //
        0.0,
        -2.0 / height,
        0.0,
        0.0, //
        0.0,
        0.0,
        -1.0,
        0.0, //
        -1.0,
        1.0,
        0.0,
        1.0,
    ])
}

/// Index order of the two triangles of quad `q` (GL_QUADS has no core-profile equivalent).
pub fn quad_indices(quads: usize) -> Vec<u32> {
    (0..quads as u32)
        .flat_map(|q| [0, 1, 2, 0, 2, 3].map(|i| q * 4 + i))
        .collect()
}

/// Flip a tightly packed bottom-up pixel buffer (`glReadPixels` order) to top-down.
pub fn flip_rows(pixels: &[u8], width: usize, height: usize, bytes_per_pixel: usize) -> Vec<u8> {
    let row = width * bytes_per_pixel;
    assert_eq!(pixels.len(), row * height, "buffer is not width*height*bpp");
    if row == 0 {
        return Vec::new();
    }
    pixels.chunks_exact(row).rev().flatten().copied().collect()
}
