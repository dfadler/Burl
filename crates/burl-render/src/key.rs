//! The file extension key (`FileKey` / `FileKeyEntry`, src/key.cpp): one gradient bar per file
//! extension with its live file count, sorted by count, sliding and fading as the counts change.
//! The sim reports `KeyEvent`s (the `file_key.inc/dec/logic` calls); `Overlays::draw` replays them
//! before drawing. Not ported: `FileKey::clear` and `colourize` (the sim has no reset or
//! recolour command yet).

use crate::{
    quad_batches, Batch, Blend, Font, FontManager, Pass, Pipeline, QuadBuf, QuadVertex, Space,
};
use burl_sim::view::KeyEvent;
use glam::{Vec2, Vec3, Vec4};
use std::collections::BTreeMap;

/// `FileKey(1.0f)`.
const UPDATE_INTERVAL: f32 = 1.0;

struct Entry {
    colour: Vec3,
    ext: String,
    display_ext: String,
    alpha: f32,
    count: i32,
    pos_y: f32,
    src_y: f32,
    dest_y: f32,
    move_elapsed: f32,
    left_margin: f32,
    width: f32,
    height: f32,
    show: bool,
}

impl Entry {
    fn new(mgr: &mut FontManager, font: &Font, ext: &str, colour: Vec3, s: &KeySettings) -> Self {
        let width = 90.0 * s.font_scale;
        let mut display_ext = ext.to_string();
        let mut truncated = false;
        while mgr.width(font, &display_ext) > width - 15.0 * s.font_scale {
            display_ext.pop();
            truncated = true;
        }
        if truncated {
            display_ext += "...";
        }
        Self {
            colour,
            ext: ext.into(),
            display_ext,
            alpha: 0.0,
            count: 0,
            pos_y: -1.0,
            src_y: -1.0,
            dest_y: -1.0,
            move_elapsed: 1.0,
            left_margin: s.scaled_font_size + 4.0,
            width,
            height: s.scaled_font_size + 4.0,
            show: true,
        }
    }

    fn is_finished(&self) -> bool {
        self.count <= 0 && self.alpha <= 0.0
    }

    fn set_dest_y(&mut self, dest_y: f32) {
        if dest_y == self.dest_y {
            return;
        }
        self.dest_y = dest_y;
        self.src_y = self.pos_y;
        self.move_elapsed = 0.0;
    }

    fn logic(&mut self, dt: f32) {
        if self.count <= 0 || !self.show {
            self.alpha = (self.alpha - dt).max(0.0);
        } else if self.alpha < 1.0 {
            self.alpha = (self.alpha + dt).min(1.0);
        }
        if self.pos_y != self.dest_y {
            if self.pos_y < 0.0 {
                self.pos_y = self.dest_y;
            } else {
                self.move_elapsed += dt;
                self.pos_y = if self.move_elapsed >= 1.0 {
                    self.dest_y
                } else {
                    self.src_y + (self.dest_y - self.src_y) * self.move_elapsed
                };
            }
        }
    }

    fn draw(&self, mgr: &mut FontManager, font: &mut Font, out: &mut Vec<Batch>) {
        if self.is_finished() {
            return;
        }
        let pos = Vec2::new(self.alpha * self.left_margin, self.pos_y);
        let (w, h, c, a) = (self.width, self.height, self.colour, self.alpha);
        let v = |x: f32, y: f32, col: Vec4| QuadVertex {
            pos: [x, y],
            colour: col.to_array(),
            uv: [0.0, 0.0],
        };
        let plain = |buf: &QuadBuf, offset: [f32; 2]| {
            quad_batches(
                Pass::Overlay,
                Pipeline::Plain,
                Blend::Alpha,
                Space::Screen,
                offset,
                buf,
                None,
            )
        };
        // the drop shadow: the bar's rectangle, translated by (3, 3)
        let mut shadow = QuadBuf::default();
        shadow.add(
            None,
            pos,
            Vec2::new(w, h),
            Vec4::new(0.0, 0.0, 0.0, a * 0.333),
            Vec4::ZERO,
        );
        out.extend(plain(&shadow, [3.0, 3.0]));
        // the bar: darker on the left, full colour on the right
        let (dark, full) = (c.extend(a) * Vec4::new(0.5, 0.5, 0.5, 1.0), c.extend(a));
        let mut bar = QuadBuf::default();
        bar.add_vertices(
            None,
            [
                v(pos.x, pos.y, dark),
                v(pos.x, pos.y + h, dark),
                v(pos.x + w, pos.y + h, full),
                v(pos.x + w, pos.y, full),
            ],
        );
        out.extend(plain(&bar, [0.0; 2]));

        font.set_colour(Vec4::new(1.0, 1.0, 1.0, a));
        font.shadow = false;
        mgr.draw(
            font,
            out,
            (pos.x as i32 + 2) as f32,
            (pos.y as i32 + 3) as f32,
            &self.display_ext,
        );
        font.shadow = true;
        mgr.draw(
            font,
            out,
            (pos.x as i32) as f32 + w + 4.0,
            (pos.y as i32 + 3) as f32,
            &self.count.to_string(),
        );
    }
}

/// What the key reads from the settings and the window.
#[derive(Clone, Copy, Debug)]
pub struct KeySettings {
    pub font_scale: f32,
    pub scaled_font_size: f32,
    pub height: f32,
    /// `--file-extension-fallback`.
    pub fallback: bool,
}

pub struct FileKey {
    font: Font,
    entries: BTreeMap<String, Entry>,
    /// `active_keys`: sorted extensions, the ones that are drawn.
    active: Vec<String>,
    interval_remaining: f32,
    show: bool,
}

impl FileKey {
    pub fn new(mgr: &mut FontManager, font_size: u32) -> Self {
        Self {
            font: mgr.grab(font_size),
            entries: BTreeMap::new(),
            active: Vec::new(),
            interval_remaining: 1.0,
            show: true,
        }
    }

    /// `FileKey::setShow`: also forces a recount on the next tick.
    pub fn set_show(&mut self, show: bool) {
        self.show = show;
        for k in &self.active {
            if let Some(e) = self.entries.get_mut(k) {
                e.show = show;
            }
        }
        self.interval_remaining = 0.0;
    }

    pub fn apply(&mut self, mgr: &mut FontManager, events: &[KeyEvent], s: &KeySettings) {
        for ev in events {
            match ev {
                KeyEvent::Tick(dt) => self.logic(*dt, s),
                KeyEvent::Inc { path, colour } => {
                    let ext = extension(path, s.fallback).to_string();
                    if !self.entries.contains_key(&ext) {
                        let e = Entry::new(mgr, &self.font, &ext, *colour, s);
                        self.entries.insert(ext.clone(), e);
                    }
                    self.entries.get_mut(&ext).expect("just inserted").count += 1;
                }
                KeyEvent::Clear => {
                    for k in &self.active {
                        if let Some(e) = self.entries.get_mut(k) {
                            e.count = 0;
                        }
                    }
                    self.interval_remaining = 0.0;
                }
                KeyEvent::Dec { path } => {
                    if let Some(e) = self.entries.get_mut(extension(path, s.fallback)) {
                        e.count -= 1;
                    }
                }
            }
        }
    }

    fn logic(&mut self, dt: f32, s: &KeySettings) {
        self.interval_remaining -= dt;
        if self.interval_remaining <= 0.0 {
            if self.show {
                let (mut active, finished): (Vec<_>, Vec<_>) =
                    self.entries.values().partition(|e| !e.is_finished());
                // by count, then name
                active.sort_by(|a, b| b.count.cmp(&a.count).then_with(|| a.ext.cmp(&b.ext)));
                let max_visible = (((s.height - 150.0) / 20.0) as i32).max(0) as usize;
                active.truncate(max_visible);
                let active: Vec<String> = active.iter().map(|e| e.ext.clone()).collect();
                let finished: Vec<String> = finished.iter().map(|e| e.ext.clone()).collect();

                let offset_y = s.scaled_font_size + 6.0;
                let mut key_y = offset_y;
                for k in &active {
                    let e = self.entries.get_mut(k).expect("active entry");
                    if e.count > 0 {
                        e.set_dest_y(key_y);
                    }
                    key_y += offset_y;
                }
                for k in finished {
                    self.entries.remove(&k);
                }
                self.active = active;
            }
            self.interval_remaining = UPDATE_INTERVAL;
        }
        for k in &self.active {
            if let Some(e) = self.entries.get_mut(k) {
                e.logic(dt);
            }
        }
    }

    pub fn draw(&mut self, mgr: &mut FontManager, out: &mut Vec<Batch>) {
        for k in &self.active {
            if let Some(e) = self.entries.get(k) {
                e.draw(mgr, &mut self.font, out);
            }
        }
    }
}

fn extension(path: &str, fallback: bool) -> &str {
    crate::labels::extension(path.rsplit('/').next().unwrap_or(path), fallback)
}
