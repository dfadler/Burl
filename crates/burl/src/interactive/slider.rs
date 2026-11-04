//! The position slider in the window (`Gource::mouseMove`/`mouseClick`/`logic`/`draw`): the
//! `burl_render::Slider` plus the mouse state and settings it needs. The exporter never has one.

use burl_conf::settings::GourceSettings;
use burl_gl::{GlContext, Renderer};
use burl_render::glam::Vec2;
use burl_render::{FontManager, Frame, Slider, PAGE_SIZE};
use burl_sim::{Command, Sim};

/// `strftime` format of `Gource::dateAtPosition`.
const DATE_FORMAT: &str = "%A, %d %B, %Y";

pub struct SliderHook {
    slider: Slider,
    hide_progress: bool,
    /// `mousepos`: where the cursor last was (physical pixels).
    mouse: Vec2,
    /// `commitlog->isSeekable()`, as of the last frame.
    seekable: bool,
    width: u32,
}

impl SliderHook {
    pub fn new(
        fonts: &mut FontManager,
        g: &GourceSettings,
        w: u32,
        h: u32,
        repo_count: i32,
    ) -> Self {
        Self {
            slider: Slider::new(fonts, g.font_scale, w as f32, h as f32, repo_count == 1),
            hide_progress: g.hide_progress,
            mouse: Vec2::ZERO,
            seekable: false,
            width: w,
        }
    }

    /// `Gource::mouseMove`: remember the position and, over the slider, caption it with the date
    /// of the commit there.
    pub fn moved(&mut self, pos: Vec2, sim: &mut Sim, fonts: &mut FontManager) {
        self.mouse = pos;
        if self.hide_progress {
            return;
        }
        if let Some(percent) = self.slider.mouse_over(pos) {
            let date = sim
                .commit_time_at(percent)
                .map(|t| burl_util::timeparse::format_local(t, DATE_FORMAT))
                .unwrap_or_default();
            self.slider.set_caption(fonts, &date);
        }
    }

    /// `Gource::mouseClick` for the left button: a seek when `canSeek` and the click is on the
    /// slider.
    pub fn click(&mut self) -> Option<Command> {
        if !self.seekable || self.hide_progress {
            return None;
        }
        self.slider.click(self.mouse).map(Command::SeekTo)
    }

    pub fn resize(&mut self, w: u32, h: u32) {
        self.width = w;
        self.slider.resize(w as f32, h as f32);
    }

    /// `slider.setPercent`/`logic`/`draw` for one frame: appends to the frame's overlay (after
    /// the key, as `Gource::draw` does) and uploads any glyphs the caption cached.
    pub fn frame<C: GlContext>(
        &mut self,
        fonts: &mut FontManager,
        r: &mut Renderer<C>,
        sim: &Sim,
        frame: &mut Frame,
        seekable: bool,
        dt: f32,
    ) -> Result<(), String> {
        self.seekable = seekable;
        self.slider.set_percent(sim.percent());
        self.slider.logic(sim.logic_dt(dt));
        if seekable && !self.hide_progress {
            self.slider
                .draw(fonts, &mut frame.overlay, self.width as f32);
            for u in fonts.take_updates() {
                r.upload_texture(u.id, PAGE_SIZE as u32, PAGE_SIZE as u32, &u.rgba, false)?;
            }
        }
        Ok(())
    }
}
