//! The text overlays drawn after the names in `Gource::draw` (src/gource.cpp:2655-2675): the date
//! (`updateTime`), the title and the captions (`Gource::logic` places them, `RCaption::draw`).
//! They go to `Frame::overlay` in that order, each with its drop shadow (`FXFont::draw` in
//! immediate mode), then the file key (`--key`). Not ported: the splash screen (never on screen
//! when exporting) and the hover text box (the slider is `slider.rs`, added by the window only).

use crate::key::FileKey;
use crate::{Batch, Font, FontManager, KeySettings};
use burl_sim::view::SceneView;
use glam::{Vec3, Vec4};

#[derive(Clone, Debug)]
pub struct OverlaySettings {
    pub width: f32,
    pub height: f32,
    pub hide_date: bool,
    /// `--hide progress` (`canSeek` is false with it).
    pub hide_progress: bool,
    pub title: String,
    pub date_format: String,
    pub font_colour: Vec3,
    pub caption_colour: Vec3,
    pub caption_offset: i32,
    /// `--key`.
    pub show_key: bool,
    pub font_scale: f32,
    pub scaled_font_size: f32,
    pub file_extension_fallback: bool,
}

/// `fontmedium` and `fontcaption` plus the state `Gource` keeps between frames.
pub struct Overlays {
    medium: Font,
    caption: Font,
    /// `date_x_offset`: only moves when the date's half width changes by more than 5 pixels.
    date_x_offset: i32,
    /// Placed captions: `(activation number, x, y)`; a caption keeps its row until it expires.
    rows: Vec<(usize, f32, f32)>,
    key: FileKey,
}

impl Overlays {
    /// `font_size` is `scaled_font_size`, `caption_size` is `--caption-size`.
    pub fn new(mgr: &mut FontManager, font_size: u32, caption_size: u32) -> Self {
        let (mut medium, mut caption) = (mgr.grab(font_size), mgr.grab(caption_size));
        medium.shadow = true;
        caption.shadow = true;
        caption.align_top = false;
        Self {
            medium,
            caption,
            date_x_offset: 0,
            rows: Vec::new(),
            key: FileKey::new(mgr, font_size),
        }
    }

    /// Appends the date, title and caption batches for one frame to `out`. `date` is `time`
    /// already formatted with `--date-format`.
    pub fn draw(
        &mut self,
        mgr: &mut FontManager,
        out: &mut Vec<Batch>,
        scene: &SceneView,
        date: &str,
        s: &OverlaySettings,
    ) {
        // Gource::logic: file_key.logic and the inc/dec calls of this frame.
        let ks = KeySettings {
            font_scale: s.font_scale,
            scaled_font_size: s.scaled_font_size,
            height: s.height,
            fallback: s.file_extension_fallback,
        };
        self.key.apply(mgr, &scene.key_events, &ks);

        // Gource::updateTime (a zero time clears the date and leaves the offset alone).
        if scene.time != 0 {
            let half = ((mgr.width(&self.medium, date) as i32) as f64 * 0.5) as i32;
            if (self.date_x_offset - half).abs() > 5 {
                self.date_x_offset = half;
            }
        }
        let date = if scene.time == 0 { "" } else { date };

        // Gource::logic: where the captions go.
        let medium_h = mgr.max_height(&self.medium);
        let caption_h = mgr.max_height(&self.caption);
        let can_seek = scene.seekable && !s.hide_progress;
        let mut start_y = if can_seek {
            // PositionSlider::resize: the slider's top edge is height - 70.
            s.height - 70.0 - 35.0
        } else {
            s.height - medium_h - 20.0
        };
        if !s.title.is_empty() {
            start_y = start_y.min(s.height - 20.0 - medium_h);
        }
        let start_y = start_y.floor();

        let first = scene.captions_activated - scene.captions.len();
        self.rows.retain(|r| r.0 >= first);
        for (i, c) in scene.captions.iter().enumerate() {
            let seq = first + i;
            if self.rows.iter().any(|r| r.0 == seq) {
                continue;
            }
            let mut y = start_y;
            while self.rows.iter().any(|r| r.2 == y) {
                y -= caption_h;
            }
            let w = mgr.width(&self.caption, &c.text);
            // `int caption_offset_x`: the float expressions truncate on assignment.
            let x = match s.caption_offset {
                0 => ((s.width as i32 / 2) as f32 - w / 2.0).trunc(),
                o if o < 0 => (s.width + o as f32 - w).trunc(),
                o => o as f32,
            };
            self.rows.push((seq, x, y));
        }

        // Gource::draw, in order.
        self.medium.set_colour(s.font_colour.extend(1.0));
        if !s.hide_date {
            mgr.draw(
                &self.medium,
                out,
                (s.width as i32 / 2 - self.date_x_offset) as f32,
                20.0,
                date,
            );
        }
        if !s.title.is_empty() {
            self.medium.align_top = false;
            mgr.draw(&self.medium, out, 10.0, s.height - 10.0, &s.title);
            self.medium.align_top = true;
        }
        for (i, c) in scene.captions.iter().enumerate() {
            let &(_, x, y) = self
                .rows
                .iter()
                .find(|r| r.0 == first + i)
                .expect("placed above");
            let col: Vec4 = s.caption_colour.extend(c.alpha);
            self.caption.set_colour(col);
            mgr.draw(&self.caption, out, x, y, &c.text);
        }
        self.key.draw(mgr, out);
        self.key.set_show(s.show_key);
    }
}

impl Overlays {
    /// The date/title font, for width checks.
    pub fn medium_font(&self) -> &Font {
        &self.medium
    }
}
