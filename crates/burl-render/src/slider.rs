//! The position slider (`PositionSlider`, src/slider.cpp): an outlined box along the bottom of the
//! window with a marker at the log position, fading in when the mouse is over it and out a second
//! after it leaves; a click seeks to that fraction of the log and the hover shows the date there.
//! Windowed only: `Gource` shows it unless exporting, with one repo (gource.cpp:167-171).
//! The 2 px `GL_LINE_STRIP`/`GL_LINES` become quads of the same width (no line batches here).

use crate::{quad_batches, Batch, Blend, Font, FontManager, Pass, Pipeline, QuadBuf, Space};
use glam::{Vec2, Vec3, Vec4};

/// `int gap = 35` in `PositionSlider::resize`.
const GAP: f32 = 35.0;
/// `glLineWidth(2.0f)` in `drawSlider`.
const LINE_WIDTH: f32 = 2.0;

pub struct Slider {
    font: Font,
    font_scale: f32,
    min: Vec2,
    max: Vec2,
    percent: f32,
    /// The mouse x while over the slider, else -1.
    mouseover: f32,
    mouseover_elapsed: f32,
    alpha: f32,
    colour: Vec3,
    caption: String,
    capwidth: f32,
}

impl Slider {
    /// `PositionSlider::init`; `show` is `exporter==0 && repo_count==1` (`slider.show()`), which
    /// starts it visible for a second instead of hidden.
    pub fn new(
        mgr: &mut FontManager,
        font_scale: f32,
        width: f32,
        height: f32,
        show: bool,
    ) -> Self {
        let mut font = mgr.grab((16.0 * font_scale) as u32);
        font.shadow = true;
        let mut s = Self {
            font,
            font_scale,
            min: Vec2::ZERO,
            max: Vec2::ZERO,
            percent: 0.0,
            mouseover: -1.0,
            mouseover_elapsed: if show { 0.0 } else { 1.0 },
            alpha: 0.0,
            colour: Vec3::ONE,
            caption: String::new(),
            capwidth: 0.0,
        };
        s.resize(width, height);
        s
    }

    /// `PositionSlider::resize`: the box is `gap` in from the sides and bottom, `gap` tall.
    pub fn resize(&mut self, width: f32, height: f32) {
        self.min = Vec2::new(GAP, height - GAP * 2.0);
        self.max = Vec2::new(width - GAP, height - GAP);
    }

    /// `Bounds2D::contains` (edges count) then the fraction of the width: the pure part of
    /// `mouseOver`/`click`.
    pub fn percent_at(&self, pos: Vec2) -> Option<f32> {
        let inside = self.min.x <= pos.x
            && self.min.y <= pos.y
            && self.max.x >= pos.x
            && self.max.y >= pos.y;
        inside.then(|| (pos.x - self.min.x) / (self.max.x - self.min.x))
    }

    /// `PositionSlider::mouseOver`: over the box it stays visible and remembers the x for the
    /// caption; off it the caption goes.
    pub fn mouse_over(&mut self, pos: Vec2) -> Option<f32> {
        let p = self.percent_at(pos);
        if p.is_some() {
            self.mouseover_elapsed = 0.0;
            self.mouseover = pos.x;
        } else {
            self.mouseover = -1.0;
        }
        p
    }

    /// `PositionSlider::click`: the fraction to seek to when `pos` is on the slider.
    pub fn click(&mut self, pos: Vec2) -> Option<f32> {
        let p = self.mouse_over(pos)?;
        self.percent = p;
        Some(p)
    }

    /// `PositionSlider::setCaption`: the date shown over the mouse.
    pub fn set_caption(&mut self, mgr: &mut FontManager, caption: &str) {
        self.caption = caption.to_string();
        self.capwidth = if caption.is_empty() {
            0.0
        } else {
            mgr.width(&self.font, caption)
        };
    }

    pub fn set_percent(&mut self, percent: f32) {
        self.percent = percent;
    }

    /// `PositionSlider::logic`: one second of fade (`fade_time`) either way.
    pub fn logic(&mut self, dt: f32) {
        if self.mouseover < 0.0 && self.mouseover_elapsed < 1.0 {
            self.mouseover_elapsed += dt;
        }
        if self.mouseover_elapsed < 1.0 && self.alpha < 1.0 {
            self.alpha = (self.alpha + dt).min(1.0);
        } else if self.mouseover_elapsed >= 1.0 && self.alpha > 0.0 {
            self.alpha = (self.alpha - dt).max(0.0);
        }
    }

    /// `PositionSlider::draw` (called when `canSeek`): the shadowed outline and marker, then the
    /// hover caption, as `Pass::Overlay` batches appended to `out`.
    pub fn draw(&mut self, mgr: &mut FontManager, out: &mut Vec<Batch>, width: f32) {
        let pos_x = self.min.x + (self.max.x - self.min.x) * self.percent;
        let a = self.alpha;
        for (offset, colour) in (a > 0.0)
            .then_some([
                ([2.0, 2.0], Vec4::new(0.0, 0.0, 0.0, 0.7 * a)),
                ([0.0; 2], self.colour.extend(a)),
            ])
            .into_iter()
            .flatten()
        {
            let mut buf = QuadBuf::default();
            for (p, q) in self.segments(pos_x) {
                line(&mut buf, p, q, colour);
            }
            out.extend(quad_batches(
                Pass::Overlay,
                Pipeline::Plain,
                Blend::Alpha,
                Space::Screen,
                offset,
                &buf,
                None,
            ));
        }
        if !self.caption.is_empty() && self.mouseover >= 0.0 {
            let y = self.min.y - (25.0 * self.font_scale).trunc();
            let x = (self.mouseover - self.capwidth / 2.0)
                .max(1.0)
                .min(width - self.capwidth - 1.0);
            self.font.set_colour(Vec4::ONE);
            mgr.draw(&self.font, out, x, y, &self.caption);
        }
    }

    /// `Bounds2D::draw` (min -> (max.x, min.y) -> max -> (min.x, max.y) -> min) and the marker.
    fn segments(&self, pos_x: f32) -> [(Vec2, Vec2); 5] {
        let (a, b) = (self.min, self.max);
        let (tr, bl) = (Vec2::new(b.x, a.y), Vec2::new(a.x, b.y));
        [
            (a, tr),
            (tr, b),
            (b, bl),
            (bl, a),
            (Vec2::new(pos_x, a.y), Vec2::new(pos_x, b.y)),
        ]
    }
}

/// An axis-aligned line `LINE_WIDTH` wide centred on `p`-`q`.
fn line(buf: &mut QuadBuf, p: Vec2, q: Vec2, colour: Vec4) {
    let h = LINE_WIDTH / 2.0;
    let (lo, hi) = (p.min(q), p.max(q));
    let grow = if p.x == q.x {
        Vec2::new(h, 0.0)
    } else {
        Vec2::new(0.0, h)
    };
    buf.add(None, lo - grow, hi - lo + grow * 2.0, colour, Vec4::ZERO);
}
