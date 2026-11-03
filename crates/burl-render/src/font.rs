//! Text: the glyph atlas and `FXFont` drawing, ported from `FXGlyph`, `FXGlyphPage`, `FXGlyphSet`,
//! `FXFont` and `FXFontManager` (reference/gource-cpp-dump/src/core/fxfont.{h,cpp}).
//!
//! Glyphs are rasterised with pure-Rust `fontdue` instead of FreeType (ticket #19 item 3), so the
//! coverage bitmaps differ from Gource's: FreeType hints the outline (FT_LOAD_DEFAULT), fontdue
//! does not. Metrics, the 512x512 page packing, texture coordinates and the quad layout are
//! ported unchanged. Advances are rounded to whole pixels, which is what hinted FreeType reports.

use crate::{quad_batches, Batch, Blend, Pass, Pipeline, QuadBuf, Space, TextureId};
use glam::{Vec2, Vec4};
use std::collections::HashMap;

/// The font Gource ships (`data/fonts/FreeSans.ttf`), the `--font-file` default.
pub const FREE_SANS: &[u8] = include_bytes!("../../../data/fonts/FreeSans.ttf");

/// `min(512, GL_MAX_TEXTURE_SIZE)`: the page size and the denominator of the text shader's
/// `texel_size` (`Gource::font_texel_size`).
pub const PAGE_SIZE: usize = 512;

/// `gGourceShadowStrength` for the text shader pass (gource.cpp:2536 passes a literal 0.7).
pub const TEXT_SHADOW_STRENGTH: f32 = 0.7;

/// `FXGlyphSet::init`'s precache list.
const PRECACHE: &str =
    "0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ;:'\",<.>/?-_=+!@#$%^&*()\\ ";

#[derive(Clone, Copy)]
struct Glyph {
    dims: Vec2,
    corner: Vec2,
    advance: f32,
    /// `None` only if the bitmap does not fit an empty page (C++ throws `FXFontException`).
    page: Option<(usize, Vec4)>,
}

struct Page {
    id: TextureId,
    data: Vec<u8>,
    dirty: bool,
    cursor: (usize, usize),
    max_glyph_height: usize,
}

impl Page {
    fn new(id: TextureId) -> Self {
        Self {
            id,
            data: vec![0; PAGE_SIZE * PAGE_SIZE],
            dirty: false,
            cursor: (1, 1),
            max_glyph_height: 1,
        }
    }

    /// `FXGlyphPage::addGlyph`: row-wrapped shelf packing with 3 pixels of padding. Returns the
    /// glyph's texture coordinates.
    fn add(&mut self, width: usize, rows: usize, bitmap: &[u8]) -> Option<Vec4> {
        let padding = 3;
        let (mut cx, mut cy) = self.cursor;
        self.max_glyph_height = self.max_glyph_height.max(rows);
        if cx + width + padding > PAGE_SIZE {
            cx = 1;
            cy += self.max_glyph_height + padding;
            if cx + width + padding > PAGE_SIZE {
                return None;
            }
        }
        if cy + rows + padding > PAGE_SIZE {
            return None;
        }
        self.dirty = true;
        for j in 0..rows {
            let dst = cx + (j + cy) * PAGE_SIZE;
            self.data[dst..dst + width].copy_from_slice(&bitmap[j * width..(j + 1) * width]);
        }
        let s = PAGE_SIZE as f32;
        let tc = Vec4::new(
            (cx as f32 - 0.5) / s,
            (cy as f32 - 0.5) / s,
            ((cx + width) as f32 + 1.5) / s,
            ((cy + rows) as f32 + 1.5) / s,
        );
        self.cursor = (cx + width + padding, cy);
        Some(tc)
    }
}

struct GlyphSet {
    size: u32,
    ascender: f32,
    descender: f32,
    glyphs: HashMap<char, Glyph>,
    pages: Vec<Page>,
}

impl GlyphSet {
    fn glyph(&mut self, font: &fontdue::Font, next_texture: &mut u32, c: char) -> Glyph {
        if !self.glyphs.contains_key(&c) {
            let (m, bitmap) = font.rasterize(c, self.size as f32);
            // FT_Glyph_To_Bitmap: left = xmin, top = ymin + rows (y up); FXGlyph adds the
            // 2 pixel border to dims and the half pixel to corner.
            let corner = Vec2::new(m.xmin as f32, -((m.ymin + m.height as i32) as f32))
                + Vec2::new(0.5, -0.5);
            let dims = Vec2::new(m.width as f32, m.height as f32) + Vec2::splat(2.0);
            let mut page = None;
            for retry in [false, true] {
                if retry || self.pages.is_empty() {
                    let id = TextureId(*next_texture);
                    *next_texture += 1;
                    self.pages.push(Page::new(id));
                }
                let i = self.pages.len() - 1;
                if let Some(tc) = self.pages[i].add(m.width, m.height, &bitmap) {
                    page = Some((i, tc));
                    break;
                }
            }
            self.glyphs.insert(
                c,
                Glyph {
                    dims,
                    corner,
                    advance: m.advance_width.round(),
                    page,
                },
            );
        }
        self.glyphs[&c]
    }
}

/// `FXFont`: the per-use drawing state. Glyph data lives in the `FontManager`.
#[derive(Clone, Debug)]
pub struct Font {
    size: u32,
    pub shadow: bool,
    pub round: bool,
    pub align_right: bool,
    pub align_top: bool,
    pub shadow_offset: Vec2,
    shadow_strength: f32,
    shadow_colour: Vec4,
    colour: Vec4,
}

impl Font {
    fn new(size: u32) -> Self {
        Self {
            size,
            shadow: false,
            round: false,
            align_right: false,
            align_top: true,
            shadow_offset: Vec2::ONE,
            shadow_strength: 0.7,
            shadow_colour: Vec4::new(0.0, 0.0, 0.0, 0.7),
            colour: Vec4::ONE,
        }
    }

    pub fn size(&self) -> u32 {
        self.size
    }

    pub fn set_colour(&mut self, colour: Vec4) {
        self.colour = colour;
        self.shadow_colour.w = colour.w * self.shadow_strength;
    }

    pub fn set_alpha(&mut self, alpha: f32) {
        self.colour.w = alpha;
        self.shadow_colour.w = self.shadow_strength * alpha;
    }

    pub fn set_shadow_strength(&mut self, s: f32) {
        self.shadow_strength = s;
        self.shadow_colour.w = self.colour.w * s;
    }
}

/// A glyph page whose pixels changed: upload as texture `id`, `PAGE_SIZE` square RGBA8, linear
/// filtering, no mipmaps, clamp to edge (the C++ uses a `GL_ALPHA` texture; white RGB with the
/// coverage in alpha samples identically for the modulate and text shaders).
pub struct PageUpdate {
    pub id: TextureId,
    pub rgba: Vec<u8>,
}

/// `FXFontManager` + the glyph sets it owns, for one font file.
pub struct FontManager {
    font: fontdue::Font,
    next_texture: u32,
    sets: HashMap<u32, GlyphSet>,
    /// `font_vbo`: text drawn with `draw_buffered`, one run per glyph page.
    pub buffer: QuadBuf,
}

impl FontManager {
    /// `first_texture` is the first `TextureId` handed to a glyph page; pages take consecutive ids.
    pub fn new(ttf: &[u8], first_texture: u32) -> Result<Self, String> {
        let font = fontdue::Font::from_bytes(ttf, fontdue::FontSettings::default())
            .map_err(|e| format!("font: {e}"))?;
        Ok(Self {
            font,
            next_texture: first_texture,
            sets: HashMap::new(),
            buffer: QuadBuf::default(),
        })
    }

    /// `FXFontManager::grab`: the glyph set for `size` is created and precached on first use.
    pub fn grab(&mut self, size: u32) -> Font {
        let size = size.max(1);
        if !self.sets.contains_key(&size) {
            let lm = self.font.horizontal_line_metrics(size as f32);
            let (ascender, descender) = lm.map_or((0.0, 0.0), |m| (m.ascent, m.descent));
            let mut set = GlyphSet {
                size,
                ascender,
                descender,
                glyphs: HashMap::new(),
                pages: Vec::new(),
            };
            for c in PRECACHE.chars() {
                set.glyph(&self.font, &mut self.next_texture, c);
            }
            self.sets.insert(size, set);
        }
        Font::new(size)
    }

    /// Pages changed since the last call, to be uploaded before the next render.
    pub fn take_updates(&mut self) -> Vec<PageUpdate> {
        let mut sets: Vec<_> = self.sets.values_mut().collect();
        sets.sort_by_key(|s| s.size);
        sets.into_iter()
            .flat_map(|s| s.pages.iter_mut())
            .filter_map(|p| {
                std::mem::take(&mut p.dirty).then(|| PageUpdate {
                    id: p.id,
                    rgba: p.data.iter().flat_map(|&a| [255, 255, 255, a]).collect(),
                })
            })
            .collect()
    }

    /// `FXFont::getWidth`: the sum of the glyph advances.
    pub fn width(&mut self, f: &Font, text: &str) -> f32 {
        let set = self.sets.get_mut(&f.size).expect("font from this manager");
        text.chars()
            .map(|c| set.glyph(&self.font, &mut self.next_texture, c).advance)
            .sum()
    }

    /// `FXFont::getHeight`: `ceil(ascender + descender)`. The descender is negative.
    pub fn height(&self, f: &Font) -> f32 {
        let s = &self.sets[&f.size];
        (s.ascender + s.descender).ceil()
    }

    /// `fontmanager.startBuffer`.
    pub fn start_buffer(&mut self) {
        self.buffer.clear();
    }

    /// `FXFont::draw` anchor handling (right/top alignment, rounding).
    fn anchor(&mut self, f: &Font, x: f32, y: f32, text: &str) -> Vec2 {
        let mut p = Vec2::new(x, y);
        if f.align_right {
            p.x -= self.width(f, text);
        }
        if f.align_top {
            p.y += self.height(f);
        }
        if f.round {
            p = p.round();
        }
        p
    }

    /// `FXGlyphSet::drawToVBO` / `FXGlyphSet::draw`'s quads: glyph quads from `cursor`.
    fn glyph_quads(
        &mut self,
        f: &Font,
        buf: &mut QuadBuf,
        origin: Vec2,
        text: &str,
        colour: Vec4,
        tabs: bool,
    ) {
        let set = self.sets.get_mut(&f.size).expect("font from this manager");
        let mut cursor = 0.0;
        for c in text.chars() {
            if tabs && c == '\t' {
                cursor += set.glyph(&self.font, &mut self.next_texture, 'M').advance * 4.0;
                continue;
            }
            let g = set.glyph(&self.font, &mut self.next_texture, c);
            if let Some((page, tc)) = g.page {
                let tex = set.pages[page].id;
                buf.add(
                    Some(tex),
                    origin + Vec2::new(cursor, 0.0) + g.corner,
                    g.dims,
                    colour,
                    tc,
                );
            }
            cursor += g.advance;
        }
    }

    /// `FXFont::draw` while `fontmanager.use_vbo` is set: the text goes to `buffer`, drawn later
    /// by the text shader pass, which adds the shadow.
    pub fn draw_buffered(&mut self, f: &Font, x: f32, y: f32, text: &str) {
        let p = self.anchor(f, x, y, text);
        let mut buf = std::mem::take(&mut self.buffer);
        self.glyph_quads(f, &mut buf, p, text, f.colour, false);
        self.buffer = buf;
    }

    /// `FXFont::draw` in immediate mode: the drop shadow (offset, `shadow_colour`) then the text,
    /// as `Pass::Overlay` batches appended to `out`.
    pub fn draw(&mut self, f: &Font, out: &mut Vec<Batch>, x: f32, y: f32, text: &str) {
        let p = self.anchor(f, x, y, text);
        let runs = [
            (f.shadow, f.shadow_offset, f.shadow_colour),
            (true, Vec2::ZERO, f.colour),
        ];
        for (on, off, colour) in runs {
            if !on {
                continue;
            }
            let mut buf = QuadBuf::default();
            self.glyph_quads(f, &mut buf, p + off, text, colour, true);
            out.extend(quad_batches(
                Pass::Overlay,
                Pipeline::Plain,
                Blend::Alpha,
                Space::Screen,
                [0.0; 2],
                &buf,
                None,
            ));
        }
    }
}
