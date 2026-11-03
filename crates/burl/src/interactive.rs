//! The windowed path (phase 6, ticket #20): the winit `ApplicationHandler` that owns the window,
//! the 16 ms accumulate loop (`SDLApp::run`, `min_delta_msec` 16 from `GourceShell`) and the
//! frame, drawn through the same `Renderer` the exporter uses. Input is not wired yet.

use super::export::upload_scene_textures;
use super::{fetch_log, log_settings, sim_settings};
use burl_conf::settings::{DisplaySettings, GourceSettings};
use burl_gl::winit::application::ApplicationHandler;
use burl_gl::winit::dpi::{LogicalSize, PhysicalPosition, PhysicalSize, Size};
use burl_gl::winit::event::WindowEvent;
use burl_gl::winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use burl_gl::winit::window::{Fullscreen, WindowAttributes, WindowId};
use burl_gl::{GlContext, Renderer, WindowedGl};
use burl_render::{build_frame, world_matrix, SceneSettings, SceneTextures};
use burl_sim::Sim;
use burl_util::prng::SimRng;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// `GourceShell`'s `min_delta_msec`: frames are at least this far apart, and a frame sees the
/// whole time accumulated since the last one (no carried remainder).
const MIN_DELTA: Duration = Duration::from_millis(16);

/// The window the display settings ask for (`SDLAppDisplay::init`).
///
/// A viewport given without `--high-dpi` asks for a drawable of exactly that many pixels (ratio
/// 1, as the C++ does), so on a Retina screen the window is physically small; anything else is a
/// logical size that the compositor scales (#20 item 3).
pub fn window_attributes(d: &DisplaySettings) -> WindowAttributes {
    let (w, h) = (d.width as u32, d.height as u32);
    let size: Size = if d.viewport_specified && !d.high_dpi {
        PhysicalSize::new(w, h).into()
    } else {
        LogicalSize::new(w, h).into()
    };
    let mut a = WindowAttributes::default()
        .with_title("Gource")
        .with_inner_size(size)
        .with_resizable(d.resizable)
        .with_decorations(!d.frameless)
        .with_transparent(d.transparent);
    if d.fullscreen {
        a = a.with_fullscreen(Some(Fullscreen::Borderless(None)));
    }
    if d.window_x >= 0 && d.window_y >= 0 {
        a = a.with_position(PhysicalPosition::new(d.window_x, d.window_y));
    }
    a
}

struct Live {
    r: Renderer<WindowedGl>,
    sim: Sim,
    tex: SceneTextures,
    ss: SceneSettings,
    clear: [f32; 4],
    last: Instant,
}

struct App {
    display: DisplaySettings,
    repo: GourceSettings,
    rng: Option<SimRng>,
    live: Option<Live>,
    error: Option<String>,
}

impl App {
    fn start(&mut self, el: &ActiveEventLoop) -> Result<Live, String> {
        let ctx = WindowedGl::new(el, window_attributes(&self.display), self.display.vsync)?;
        let (w, h) = ctx.size();
        let mut r = Renderer::new(ctx)?;
        let tex = upload_scene_textures(&mut r)?;

        let g = &mut self.repo;
        g.apply_shell_fixups(1, false);
        // The camera aspect follows the real drawable, which HiDPI makes differ from `-WxH`.
        let mut d = self.display.clone();
        d.width = w as i32;
        d.height = h as i32;
        let log = fetch_log(&g.path.clone(), Arc::new(log_settings(g))).map_err(|e| e.0)?;
        let rng = self.rng.take().ok_or("simulation already started")?;
        let mut settings = sim_settings(&d, g, 1, 1);
        settings.exporting = false;
        let sim = Sim::new(settings, log, rng).map_err(|e| e.0)?;

        let mut ss = SceneSettings::new(w, h);
        ss.bloom_multiplier = g.bloom_multiplier;
        ss.bloom_intensity = g.bloom_intensity;
        ss.hide_bloom = g.hide_bloom;
        ss.hide_tree = g.hide_tree;
        ss.hide_files = g.hide_files;
        ss.hide_users = g.hide_users;
        ss.user_scale = g.user_scale;
        let bg = g.background_colour;
        Ok(Live {
            r,
            sim,
            tex,
            ss,
            clear: [bg[0], bg[1], bg[2], 1.0],
            last: Instant::now(),
        })
    }

    /// One frame if 16 ms have passed; returns false when the simulation is finished.
    fn frame(live: &mut Live) -> Result<bool, String> {
        let now = Instant::now();
        let dt = now - live.last;
        if dt < MIN_DELTA {
            return Ok(true);
        }
        live.last = now;
        live.sim
            .update_realtime(dt.as_secs_f32())
            .map_err(|e| e.0)?;
        let scene = live.sim.scene_view();
        let frame = build_frame(&scene, &live.tex, &live.ss);
        let world = world_matrix(&scene, &live.ss);
        live.r.render(live.clear, &frame.batches(), world)?;
        live.r.blit_to_screen()?;
        live.r.context().present()?;
        let done = live.sim.is_finished();
        live.sim.end_frame();
        Ok(!done)
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, el: &ActiveEventLoop) {
        if self.live.is_some() {
            return;
        }
        match self.start(el) {
            Ok(l) => self.live = Some(l),
            Err(e) => {
                self.error = Some(e);
                el.exit();
            }
        }
    }

    fn window_event(&mut self, el: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        let Some(live) = self.live.as_mut() else {
            return;
        };
        match event {
            WindowEvent::CloseRequested => el.exit(),
            WindowEvent::Resized(s) if s.width > 0 && s.height > 0 => {
                live.r.context().resize(s.width, s.height);
                if let Err(e) = live.r.resize(s.width, s.height) {
                    self.error = Some(e);
                    el.exit();
                }
                live.ss.width = s.width as f32;
                live.ss.height = s.height as f32;
            }
            _ => {}
        }
    }

    fn about_to_wait(&mut self, el: &ActiveEventLoop) {
        let Some(live) = self.live.as_mut() else {
            return;
        };
        match Self::frame(live) {
            Ok(true) => el.set_control_flow(ControlFlow::WaitUntil(Instant::now() + MIN_DELTA)),
            Ok(false) => el.exit(),
            Err(e) => {
                self.error = Some(e);
                el.exit();
            }
        }
    }
}

/// Open the window for the first `[gource]` section and run until it is closed or the log ends.
pub fn run(
    display: DisplaySettings,
    mut repos: Vec<GourceSettings>,
    rng: SimRng,
) -> Result<(), String> {
    let event_loop = EventLoop::new().map_err(|e| format!("could not open a display: {e}"))?;
    let mut app = App {
        display,
        repo: repos.swap_remove(0),
        rng: Some(rng),
        live: None,
        error: None,
    };
    event_loop.run_app(&mut app).map_err(|e| e.to_string())?;
    app.error.map_or(Ok(()), Err)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn size(a: &WindowAttributes) -> Size {
        a.inner_size.unwrap()
    }

    #[test]
    fn viewport_without_high_dpi_is_exact_pixels() {
        let d = DisplaySettings {
            width: 640,
            height: 480,
            viewport_specified: true,
            ..DisplaySettings::default()
        };
        assert_eq!(
            size(&window_attributes(&d)),
            PhysicalSize::new(640, 480).into()
        );
    }

    #[test]
    fn default_and_high_dpi_windows_are_logical() {
        let mut d = DisplaySettings::default();
        let logical = LogicalSize::new(d.width as u32, d.height as u32);
        assert_eq!(size(&window_attributes(&d)), logical.into());
        d.viewport_specified = true;
        d.high_dpi = true;
        assert_eq!(size(&window_attributes(&d)), logical.into());
    }
}
