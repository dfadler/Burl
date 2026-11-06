//! The windowed path (phase 6, ticket #20): the winit `ApplicationHandler` that owns the window,
//! the 16 ms accumulate loop (`SDLApp::run`, `min_delta_msec` 16 from `GourceShell`) and the
//! frame, drawn through the same `Renderer` the exporter uses. Window events become `Command`s
//! in [`input`]; they queue up and are applied before the next step.

mod input;
mod slider;

use super::export::upload_scene_textures;
use super::frame::{install_avatars, new_fonts, upload_avatars, FrameKit, SharedAvatars};
use super::{fetch_log, log_settings, sim_settings};
use burl_conf::settings::{DisplaySettings, GourceSettings};
use burl_gl::winit::application::ApplicationHandler;
use burl_gl::winit::dpi::{LogicalSize, PhysicalPosition, PhysicalSize, Size};
use burl_gl::winit::event::{DeviceEvent, DeviceId, ElementState, MouseButton, WindowEvent};
use burl_gl::winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use burl_gl::winit::monitor::MonitorHandle;
use burl_gl::winit::platform::modifier_supplement::KeyEventExtModifierSupplement;
use burl_gl::winit::window::{CursorGrabMode, Fullscreen, WindowAttributes, WindowId};
use burl_gl::{monitor_index, GlContext, Renderer, WindowedGl};
use burl_render::glam::Vec2;
use burl_render::{world_matrix, FontManager, SceneSettings, SceneTextures, TextBox, PAGE_SIZE};
use burl_sim::{Command, Sim};
use burl_util::prng::SimRng;
use input::{button_command, key_action, wheel_command, Action, Arrows};
use slider::SliderHook;
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
    textbox: TextBox,
    slider: SliderHook,
    avatars: Option<SharedAvatars>,
    clear: [f32; 4],
    last: Instant,
    /// Commands since the last frame, applied in order before the step.
    pending: Vec<Command>,
    arrows: Arrows,
    alt: bool,
    disable_input: bool,
    /// `hide_mouse`: all mouse input is ignored and the cursor is hidden.
    hide_mouse: bool,
    grabbed: bool,
    /// Return was pressed with several sections: leave this one now (`GourceShell::next`).
    next: bool,
}

/// What one section needs on screen; replaced as a whole when the next section starts.
struct Loaded {
    sim: Sim,
    ss: SceneSettings,
    fonts: FontManager,
    kit: FrameKit,
    textbox: TextBox,
    slider: SliderHook,
    avatars: Option<SharedAvatars>,
    clear: [f32; 4],
    disable_input: bool,
    hide_mouse: bool,
}

/// `GourceShell`'s walk over the `[gource]` sections: each is taken from the untouched imported
/// settings (as `getNext` re-imports), and with several the walk wraps to the first after the
/// last, so a windowed run only ends by quitting. A single section is played once.
struct Sections {
    repos: Vec<GourceSettings>,
    idx: usize,
}

impl Sections {
    /// The next section's 1-based number and settings with the shell fixups applied.
    fn next(&mut self) -> Option<(i32, GourceSettings)> {
        let n = self.repos.len();
        let mut g = self.repos.get(self.idx)?.clone();
        g.apply_shell_fixups(n, false);
        let section = self.idx as i32 + 1;
        self.idx += 1;
        if self.idx == n && n > 1 {
            self.idx = 0;
        }
        Some((section, g))
    }
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
            Action::ToggleRoot => self.ss.hide_root = !self.ss.hide_root,
            Action::ToggleLabel(l) => self.kit.toggle(l),
            Action::ToggleUsers => {
                self.ss.hide_users = !self.ss.hide_users;
                self.sim.set_hide(self.ss.hide_users, self.ss.hide_files);
            }
            // ignored while a button is down or dragging
            Action::ToggleMouse if !self.sim.pointer_busy() => {
                self.hide_mouse = !self.hide_mouse;
                self.sim.set_hide_mouse(self.hide_mouse);
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
            Action::NextSection => self.next = true,
        }
    }
}

/// Whether a mouse button event reaches the sim: nothing under `--disable-input`, and nothing
/// while `M` hides the mouse, except releases (a dropped release would leave the button and the
/// grab stuck).
fn button_passes(disable_input: bool, hide_mouse: bool, state: ElementState) -> bool {
    !disable_input && (!hide_mouse || state == ElementState::Released)
}

/// Whether the timeline slider follows the pointer: not while the mouse is hidden or grabbed.
fn slider_follows(hide_mouse: bool, grabbed: bool) -> bool {
    !hide_mouse && !grabbed
}

/// The sim's pointer pixel. winit reports physical pixels and the drawable is physical too,
/// so there is no scale to apply: Gource mixes points and pixels under `--high-dpi`, Burl doesn't.
fn pointer_px(p: PhysicalPosition<f64>) -> (i32, i32) {
    (p.x as i32, p.y as i32)
}

struct App {
    display: DisplaySettings,
    sections: Sections,
    /// The generator and `first_rand_frame` between sections.
    carry: Option<(SimRng, i64)>,
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
        let l = self.load_section(&mut r, w, h, ctx_scale)?;
        // `Cursor` starts hidden under --hide-mouse
        if l.hide_mouse {
            r.context().window().set_cursor_visible(false);
        }
        Ok(Live {
            r,
            sim: l.sim,
            tex,
            ss: l.ss,
            fonts: l.fonts,
            kit: l.kit,
            textbox: l.textbox,
            slider: l.slider,
            avatars: l.avatars,
            clear: l.clear,
            last: Instant::now(),
            pending: Vec::new(),
            arrows: Arrows::default(),
            alt: false,
            disable_input: l.disable_input,
            hide_mouse: l.hide_mouse,
            grabbed: false,
            next: false,
        })
    }

    /// `GourceShell::getNext`: the section after the one that ended. `Ok(false)` when there is
    /// none (a single section, played once). Loading blocks the event loop while the log is read,
    /// as `getNext` blocks the SDL loop in the C++ original.
    fn advance(&mut self, live: &mut Live) -> Result<bool, String> {
        let carry = live.sim.take_carry();
        self.carry = Some(carry);
        let ctx = live.r.context();
        let (w, h) = ctx.size();
        let scale = ctx.scale_factor();
        if self.sections.repos.len() == 1 {
            return Ok(false);
        }
        let l = self.load_section(&mut live.r, w, h, scale)?;
        live.sim = l.sim;
        live.ss = l.ss;
        live.fonts = l.fonts;
        live.kit = l.kit;
        live.textbox = l.textbox;
        live.slider = l.slider;
        live.avatars = l.avatars;
        live.clear = l.clear;
        live.disable_input = l.disable_input;
        // the new sim holds no drag, so release a grab the old one left
        if live.grabbed {
            live.set_grab(false);
        }
        live.hide_mouse = l.hide_mouse;
        live.next = false;
        live.pending.clear();
        let hidden = live.hide_mouse || live.grabbed;
        live.window().set_cursor_visible(!hidden);
        Ok(true)
    }

    /// Build the next section's simulation and scene settings (`Gource::init`), continuing the
    /// generator from the previous section.
    fn load_section(
        &mut self,
        r: &mut Renderer<WindowedGl>,
        w: u32,
        h: u32,
        ctx_scale: f64,
    ) -> Result<Loaded, String> {
        let repo_count = self.sections.repos.len() as i32;
        let (section, mut g) = self.sections.next().ok_or("no [gource] section to play")?;
        let g = &mut g;
        if g.default_font_scale {
            g.font_scale = font_scale(dpi_ratio(&self.display, ctx_scale), h);
            g.set_scaled_font_sizes();
        }
        // The camera aspect follows the real drawable, which HiDPI makes differ from `-WxH`.
        let mut d = self.display.clone();
        d.width = w as i32;
        d.height = h as i32;
        let log = fetch_log(&g.path.clone(), Arc::new(log_settings(g))).map_err(|e| e.0)?;
        let (rng, first_rand) = self.carry.take().ok_or("simulation already started")?;
        let mut settings = sim_settings(&d, g, section, repo_count);
        settings.exporting = false;
        let avatars = install_avatars(g, &mut settings)?;
        let mut sim = Sim::new(settings, log, rng).map_err(|e| e.0)?;
        sim.carry_first_rand_frame(first_rand);

        let mut ss = SceneSettings::new(w, h);
        ss.bloom_multiplier = g.bloom_multiplier;
        ss.bloom_intensity = g.bloom_intensity;
        ss.hide_bloom = g.hide_bloom;
        ss.hide_tree = g.hide_tree;
        ss.hide_files = g.hide_files;
        ss.hide_users = g.hide_users;
        ss.hide_root = g.hide_root;
        ss.fixed_user_size = g.fixed_user_size;
        ss.user_scale = g.user_scale;
        let mut fonts = new_fonts(&g.font_file)?;
        let kit = FrameKit::new(r, &mut fonts, g, w, h)?;
        let textbox = TextBox::new(&mut fonts, (18.0 * g.font_scale) as u32);
        let slider = SliderHook::new(&mut fonts, g, w, h, repo_count);
        let bg = g.background_colour;
        Ok(Loaded {
            sim,
            ss,
            fonts,
            kit,
            slider,
            avatars,
            textbox,
            clear: [bg[0], bg[1], bg[2], 1.0],
            disable_input: g.disable_input,
            hide_mouse: g.hide_mouse,
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
        // `Gource::draw` starts with `mousetrace`; a click on the background grabs the mouse
        live.sim.draw_pointer();
        live.sim.tick_cursor(dt.as_secs_f32());
        if live.sim.is_grabbed() != live.grabbed {
            live.set_grab(live.sim.is_grabbed());
        }
        upload_avatars(&live.avatars, &mut live.r)?;
        let scene = live.sim.scene_view();
        let mut frame =
            live.kit
                .compose(&mut live.fonts, &mut live.r, &scene, &live.tex, &live.ss)?;
        live.slider.frame(
            &mut live.fonts,
            &mut live.r,
            &live.sim,
            &mut frame,
            scene.seekable,
            dt.as_secs_f32(),
        )?;
        // the hover text box comes last in `Gource::draw`, after the overlays
        if let Some(h) = live.sim.hover_text() {
            live.textbox.draw(
                &mut live.fonts,
                &mut frame.overlay,
                &h.lines,
                h.colour,
                live.sim.mouse_position(),
                Vec2::new(live.ss.width, live.ss.height),
            );
            for u in live.fonts.take_updates() {
                live.r
                    .upload_texture(u.id, PAGE_SIZE as u32, PAGE_SIZE as u32, &u.rgba, false)?;
            }
        }
        let world = world_matrix(&scene, &live.ss);
        live.r.render(live.clear, &frame.batches(), world)?;
        live.r.blit_to_screen()?;
        live.r.context().present()?;
        let done = live.sim.is_finished() || live.next;
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
                    // C++: only with more than one section
                    Some(Action::NextSection) if self.sections.repos.len() > 1 => live.next = true,
                    Some(Action::NextSection) => {}
                    Some(a) => live.act(a),
                    None => {}
                }
            }
            WindowEvent::MouseWheel { delta, .. } if !live.disable_input => {
                live.pending.extend(wheel_command(&delta));
            }
            WindowEvent::MouseInput { state, button, .. }
                if button_passes(live.disable_input, live.hide_mouse, state) =>
            {
                // `Gource::mouseClick`: a left press on the slider seeks (then also starts the
                // background drag, as in the C++)
                if button == MouseButton::Left && state == ElementState::Pressed {
                    live.pending.extend(live.slider.click());
                }
                live.pending
                    .extend(button_command(button, state == ElementState::Pressed));
            }
            WindowEvent::CursorMoved { position, .. } if !live.disable_input => {
                if slider_follows(live.hide_mouse, live.grabbed) {
                    let pos = Vec2::new(position.x as f32, position.y as f32);
                    live.slider.moved(pos, &mut live.sim, &mut live.fonts);
                }
                let (x, y) = pointer_px(position);
                live.pending.push(Command::PointerAt { x, y });
            }
            WindowEvent::CursorEntered { .. } => live.sim.set_cursor_focus(true),
            WindowEvent::CursorLeft { .. } => live.sim.set_cursor_focus(false),
            WindowEvent::Resized(s) if s.width > 0 && s.height > 0 => {
                live.r.context().resize(s.width, s.height);
                if let Err(e) = live.r.resize(s.width, s.height) {
                    self.error = Some(e);
                    el.exit();
                }
                live.ss.width = s.width as f32;
                live.ss.height = s.height as f32;
                live.sim.set_display_size(s.width as i32, s.height as i32);
                live.slider.resize(s.width, s.height);
                live.kit.resize(s.width, s.height);
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
            live.pending.push(Command::PointerDelta {
                dx: dx as f32,
                dy: dy as f32,
            });
        }
    }

    fn about_to_wait(&mut self, el: &ActiveEventLoop) {
        let Some(mut live) = self.live.take() else {
            return;
        };
        let step = match Self::frame(&mut live) {
            Ok(false) => self.advance(&mut live),
            r => r,
        };
        self.live = Some(live);
        match step {
            Ok(true) => el.set_control_flow(ControlFlow::WaitUntil(Instant::now() + MIN_DELTA)),
            Ok(false) => el.exit(),
            Err(e) => {
                self.error = Some(e);
                el.exit();
            }
        }
    }
}

/// Open the window and play the `[gource]` sections in turn until it is closed or, with a single
/// section, the log ends.
pub fn run(
    display: DisplaySettings,
    repos: Vec<GourceSettings>,
    rng: SimRng,
) -> Result<(), String> {
    let event_loop = EventLoop::new().map_err(|e| format!("could not open a display: {e}"))?;
    let mut app = App {
        display,
        sections: Sections { repos, idx: 0 },
        carry: Some((rng, -1)),
        live: None,
        error: None,
    };
    event_loop.run_app(&mut app).map_err(|e| e.to_string())?;
    app.error.map_or(Ok(()), Err)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sections(n: usize) -> Sections {
        let repos = (0..n)
            .map(|i| GourceSettings {
                title: format!("s{}", i + 1),
                ..GourceSettings::default()
            })
            .collect();
        Sections { repos, idx: 0 }
    }

    #[test]
    fn a_single_section_plays_once() {
        let mut s = sections(1);
        let (n, g) = s.next().unwrap();
        assert_eq!(n, 1);
        // no 60-second stop for a lone repo, and the window never exports
        assert!(g.stop_at_time <= 0.0 && !g.stop_at_end);
        assert!(s.next().is_none());
    }

    #[test]
    fn several_sections_cycle_with_the_sixty_second_rule() {
        let mut s = sections(3);
        let got: Vec<_> = (0..7).map(|_| s.next().unwrap()).collect();
        let order: Vec<_> = got.iter().map(|(n, _)| *n).collect();
        assert_eq!(order, [1, 2, 3, 1, 2, 3, 1]);
        assert!(got.iter().all(|(_, g)| g.stop_at_time == 60.0));
        assert_eq!(got[4].1.title, "s2");
        // the walk works on copies: the imported settings keep no stop time
        assert!(s.repos.iter().all(|g| g.stop_at_time <= 0.0));
    }

    #[test]
    fn a_stop_time_set_in_a_section_is_kept() {
        let mut s = sections(2);
        s.repos[0].stop_at_time = 5.0;
        assert_eq!(s.next().unwrap().1.stop_at_time, 5.0);
        assert_eq!(s.next().unwrap().1.stop_at_time, 60.0);
    }

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

    #[test]
    fn button_events_are_gated_by_disable_input_and_hidden_mouse_but_releases_pass() {
        use ElementState::{Pressed, Released};
        assert!(button_passes(false, false, Pressed));
        assert!(!button_passes(true, false, Pressed));
        assert!(!button_passes(true, false, Released));
        assert!(!button_passes(false, true, Pressed));
        assert!(button_passes(false, true, Released));
        assert!(!button_passes(true, true, Released));
    }

    #[test]
    fn the_slider_ignores_a_hidden_or_grabbed_pointer() {
        assert!(slider_follows(false, false));
        assert!(!slider_follows(true, false));
        assert!(!slider_follows(false, true));
    }

    #[test]
    fn the_pointer_is_physical_pixels_whatever_the_scale_factor() {
        // a Retina pointer at (1001.9, 599.2) physical is pixel (1001, 599): no halving
        assert_eq!(
            pointer_px(PhysicalPosition::new(1001.9, 599.2)),
            (1001, 599)
        );
        assert_eq!(pointer_px(PhysicalPosition::new(0.0, 0.0)), (0, 0));
    }
}
