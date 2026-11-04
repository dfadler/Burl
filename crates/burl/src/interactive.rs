//! The windowed path (phase 6, ticket #20): the winit `ApplicationHandler` that owns the window,
//! the 16 ms accumulate loop (`SDLApp::run`, `min_delta_msec` 16 from `GourceShell`) and the
//! frame, drawn through the same `Renderer` the exporter uses. Window events become `Command`s
//! in [`input`]; they queue up and are applied before the next step.

mod input;

use super::export::upload_scene_textures;
use super::frame::{new_fonts, FrameKit};
use super::{fetch_log, log_settings, sim_settings};
use burl_conf::settings::{DisplaySettings, GourceSettings};
use burl_gl::winit::application::ApplicationHandler;
use burl_gl::winit::dpi::{LogicalSize, PhysicalPosition, PhysicalSize, Size};
use burl_gl::winit::event::{DeviceEvent, DeviceId, ElementState, WindowEvent};
use burl_gl::winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use burl_gl::winit::monitor::MonitorHandle;
use burl_gl::winit::platform::modifier_supplement::KeyEventExtModifierSupplement;
use burl_gl::winit::window::{CursorGrabMode, Fullscreen, WindowAttributes, WindowId};
use burl_gl::{monitor_index, GlContext, Renderer, WindowedGl};
use burl_render::{world_matrix, FontManager, SceneSettings, SceneTextures};
use burl_sim::{Command, Sim};
use burl_util::prng::SimRng;
use input::{key_action, wheel_command, Action, Arrows, Pointer, PointerOut};
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

/// `--screen`: fullscreen goes fullscreen on that monitor; a window with no `--window-position`
/// opens at that monitor's origin (`SDL_WINDOWPOS_UNDEFINED_DISPLAY`). An explicit
/// `--window-position` stays absolute, as the C++ applies it after creation.
fn on_monitor(a: WindowAttributes, m: MonitorHandle, d: &DisplaySettings) -> WindowAttributes {
    if d.fullscreen {
        a.with_fullscreen(Some(Fullscreen::Borderless(Some(m))))
    } else if d.window_x < 0 || d.window_y < 0 {
        a.with_position(m.position())
    } else {
        a
    }
}

/// `viewport_dpi_ratio.x`: a viewport given without `--high-dpi` is an exact-pixel window (ratio
/// 1, see [`window_attributes`]); any other window is logical, so its ratio is the scale factor.
fn dpi_ratio(d: &DisplaySettings, scale_factor: f64) -> f32 {
    if d.viewport_specified && !d.high_dpi {
        1.0
    } else {
        scale_factor as f32
    }
}

/// The default `font_scale` (`Gource::init`): the DPI ratio when above 1, else one more per full
/// 1600 pixels of drawable height (integer division).
fn font_scale(ratio: f32, height: u32) -> f32 {
    if ratio > 1.0 {
        ratio
    } else {
        (1 + height as i32 / 1600) as f32
    }
}

struct Live {
    r: Renderer<WindowedGl>,
    sim: Sim,
    tex: SceneTextures,
    ss: SceneSettings,
    fonts: FontManager,
    kit: FrameKit,
    clear: [f32; 4],
    last: Instant,
    /// Commands since the last frame, applied in order before the step.
    pending: Vec<Command>,
    arrows: Arrows,
    pointer: Pointer,
    alt: bool,
    disable_input: bool,
    /// `hide_mouse`: all mouse input is ignored and the cursor is hidden.
    hide_mouse: bool,
    grabbed: bool,
}

impl Live {
    fn window(&self) -> &burl_gl::winit::window::Window {
        self.r.context().window()
    }

    /// `Gource::grabMouse`: confine the cursor while a button drags, and hide it.
    fn set_grab(&mut self, on: bool) {
        let w = self.window();
        if on {
            // Locked is not available everywhere (X11 has only Confined)
            if w.set_cursor_grab(CursorGrabMode::Locked).is_err() {
                let _ = w.set_cursor_grab(CursorGrabMode::Confined);
            }
            w.set_cursor_visible(false);
        } else {
            let _ = w.set_cursor_grab(CursorGrabMode::None);
            w.set_cursor_visible(!self.hide_mouse);
        }
        self.grabbed = on;
    }

    fn act(&mut self, a: Action) {
        match a {
            Action::Sim(c) => self.pending.push(c),
            Action::Quit => {}
            Action::ToggleTree => self.ss.hide_tree = !self.ss.hide_tree,
            Action::ToggleUsers => self.ss.hide_users = !self.ss.hide_users,
            // ignored while a button is down or dragging
            Action::ToggleMouse if !self.pointer.busy() => {
                self.hide_mouse = !self.hide_mouse;
                self.window()
                    .set_cursor_visible(!self.hide_mouse && !self.grabbed);
            }
            Action::ToggleMouse => {}
            Action::ToggleFrame => {
                let w = self.window();
                w.set_decorations(!w.is_decorated());
            }
            Action::ToggleFullscreen => {
                let w = self.window();
                let on = w.fullscreen().is_none();
                w.set_fullscreen(on.then_some(Fullscreen::Borderless(None)));
            }
        }
    }
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
        let mut attrs = window_attributes(&self.display);
        let monitors: Vec<_> = el.available_monitors().collect();
        if let Some(m) = monitor_index(self.display.screen, monitors.len()).map(|i| &monitors[i]) {
            attrs = on_monitor(attrs, m.clone(), &self.display);
        }
        let ctx = WindowedGl::new(el, attrs, self.display.vsync, self.display.multisample)?;
        let (w, h) = ctx.size();
        let ctx_scale = ctx.scale_factor();
        let mut r = Renderer::new(ctx)?;
        let tex = upload_scene_textures(&mut r)?;

        let g = &mut self.repo;
        g.apply_shell_fixups(1, false);
        if g.default_font_scale {
            g.font_scale = font_scale(dpi_ratio(&self.display, ctx_scale), h);
            g.set_scaled_font_sizes();
        }
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
        let mut fonts = new_fonts()?;
        let kit = FrameKit::new(&mut r, &mut fonts, g, w, h)?;
        let bg = g.background_colour;
        // `Cursor` starts hidden under --hide-mouse
        if g.hide_mouse {
            r.context().window().set_cursor_visible(false);
        }
        Ok(Live {
            r,
            sim,
            tex,
            ss,
            fonts,
            kit,
            clear: [bg[0], bg[1], bg[2], 1.0],
            last: Instant::now(),
            pending: Vec::new(),
            arrows: Arrows::default(),
            pointer: Pointer::default(),
            alt: false,
            disable_input: g.disable_input,
            hide_mouse: g.hide_mouse,
            grabbed: false,
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
        live.pending.extend(live.arrows.command());
        for c in live.pending.drain(..) {
            live.sim.apply(c);
        }
        live.sim
            .update_realtime(dt.as_secs_f32())
            .map_err(|e| e.0)?;
        let scene = live.sim.scene_view();
        let frame = live
            .kit
            .compose(&mut live.fonts, &mut live.r, &scene, &live.tex, &live.ss)?;
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
            WindowEvent::ModifiersChanged(m) => live.alt = m.state().alt_key(),
            // like SDL's keyboard state, a focus loss releases every held key
            WindowEvent::Focused(false) => live.arrows = Arrows::default(),
            WindowEvent::KeyboardInput {
                event,
                is_synthetic: false,
                ..
            } => {
                let key = event.key_without_modifiers();
                let down = event.state == ElementState::Pressed;
                // `Gource::logic` polls the arrows every frame, `disable_input` or not
                live.arrows.set(&key, down);
                if !down {
                    return;
                }
                let action = key_action(
                    &key,
                    event.physical_key,
                    event.location,
                    event.repeat,
                    live.alt,
                    live.disable_input,
                );
                match action {
                    Some(Action::Quit) => el.exit(),
                    Some(a) => live.act(a),
                    None => {}
                }
            }
            WindowEvent::MouseWheel { delta, .. } if !live.disable_input => {
                live.pending.extend(wheel_command(&delta));
            }
            // releases always pass: `M` can hide the mouse mid-drag, and a dropped release would
            // leave the button (and the grab) stuck
            WindowEvent::MouseInput { state, button, .. }
                if !live.disable_input && (!live.hide_mouse || state == ElementState::Released) =>
            {
                for o in live.pointer.button(button, state == ElementState::Pressed) {
                    match o {
                        PointerOut::Cmd(c) => live.pending.push(c),
                        PointerOut::Grab(on) => live.set_grab(on),
                    }
                }
            }
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

    fn device_event(&mut self, _: &ActiveEventLoop, _: DeviceId, event: DeviceEvent) {
        // Relative motion (`xrel`, `yrel`): `CursorMoved` stops while the cursor is grabbed.
        let (Some(live), DeviceEvent::MouseMotion { delta: (dx, dy) }) =
            (self.live.as_mut(), event)
        else {
            return;
        };
        if !live.disable_input && !live.hide_mouse {
            live.pending
                .extend(live.pointer.motion(dx as f32, dy as f32));
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
    fn deviation_g3_viewport_without_high_dpi_is_exact_pixels() {
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
    fn font_scale_follows_the_dpi_ratio_then_the_height() {
        let exact = DisplaySettings {
            viewport_specified: true,
            ..DisplaySettings::default()
        };
        // exact pixels never use the scale factor, even on a 2x screen
        assert_eq!(dpi_ratio(&exact, 2.0), 1.0);
        assert_eq!(dpi_ratio(&DisplaySettings::default(), 2.0), 2.0);
        assert_eq!(
            dpi_ratio(
                &DisplaySettings {
                    high_dpi: true,
                    ..exact
                },
                2.0
            ),
            2.0
        );
        assert_eq!(font_scale(2.0, 720), 2.0);
        assert_eq!(font_scale(1.0, 1599), 1.0);
        assert_eq!(font_scale(1.0, 1600), 2.0);
        assert_eq!(font_scale(1.0, 3200), 3.0);
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
