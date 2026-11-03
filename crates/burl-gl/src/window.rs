//! Windowed context (phase 6, ticket #20): a winit window with a glutin surface and a 3.2 core
//! context, pinned per the #15 spike. The app crate owns the event loop; this only builds the
//! window inside `ApplicationHandler::resumed` and is the `GlContext` the renderer draws through.

use crate::{GlContext, Result};
use glutin::config::ConfigTemplateBuilder;
use glutin::context::{ContextApi, ContextAttributesBuilder, PossiblyCurrentContext, Version};
use glutin::display::GetGlDisplay;
use glutin::prelude::*;
use glutin::surface::{Surface, SwapInterval, WindowSurface};
use glutin_winit::{DisplayBuilder, GlWindow};
use raw_window_handle::HasWindowHandle;
use std::cell::Cell;
use std::num::NonZeroU32;
use winit::event_loop::ActiveEventLoop;
use winit::window::{Window, WindowAttributes};

// Field order is drop order: the glow handle, then the surface, then the context, and the
// window last (a surface must not outlive its window).
pub struct WindowedGl {
    gl: glow::Context,
    surface: Surface<WindowSurface>,
    ctx: PossiblyCurrentContext,
    window: Window,
    size: Cell<(u32, u32)>,
}

impl WindowedGl {
    /// Create the window from `attrs` and a current 3.2 core context on it. `vsync` waits for the
    /// display refresh on every [`GlContext::present`].
    pub fn new(event_loop: &ActiveEventLoop, attrs: WindowAttributes, vsync: bool) -> Result<Self> {
        let template = ConfigTemplateBuilder::new().with_alpha_size(8);
        let (window, config) = DisplayBuilder::new()
            .with_window_attributes(Some(attrs))
            .build(event_loop, template, |mut configs| {
                // The iterator is non-empty whenever `find_configs` succeeded; glutin-winit's picker
                // has no way to return an error, so take the first.
                configs.next().expect("glutin returned no GL config")
            })
            .map_err(|e| format!("window and GL config: {e}"))?;
        let window = window.ok_or("no window was created")?;
        let display = config.display();
        let handle = window.window_handle().map_err(|e| e.to_string())?.as_raw();
        let attrs = ContextAttributesBuilder::new()
            .with_profile(glutin::context::GlProfile::Core)
            .with_context_api(ContextApi::OpenGl(Some(Version::new(3, 2))))
            .build(Some(handle));
        // SAFETY: `handle` is the live window created above; `window` is stored in the same
        // struct and dropped after the context and surface.
        let ctx = unsafe { display.create_context(&config, &attrs) }
            .map_err(|e| format!("create_context (3.2 core, no GLES fallback): {e}"))?;
        let surface_attrs = window
            .build_surface_attributes(Default::default())
            .map_err(|e| format!("surface attributes: {e}"))?;
        // SAFETY: as above; the attributes come from this window.
        let surface = unsafe { display.create_window_surface(&config, &surface_attrs) }
            .map_err(|e| format!("create_window_surface: {e}"))?;
        let ctx = ctx
            .make_current(&surface)
            .map_err(|e| format!("make_current: {e}"))?;
        let interval = if vsync {
            SwapInterval::Wait(NonZeroU32::MIN)
        } else {
            SwapInterval::DontWait
        };
        surface
            .set_swap_interval(&ctx, interval)
            .map_err(|e| format!("set_swap_interval: {e}"))?;
        // SAFETY: the context is current on this thread, and `display` outlives the loader calls.
        let gl =
            unsafe { glow::Context::from_loader_function_cstr(|s| display.get_proc_address(s)) };
        let size = window.inner_size();
        Ok(Self {
            gl,
            surface,
            ctx,
            window,
            size: Cell::new((size.width, size.height)),
        })
    }

    pub fn window(&self) -> &Window {
        &self.window
    }

    /// Follow a window resize (physical pixels). Zero sizes (minimised) are ignored.
    pub fn resize(&self, width: u32, height: u32) {
        if let (Some(w), Some(h)) = (NonZeroU32::new(width), NonZeroU32::new(height)) {
            self.surface.resize(&self.ctx, w, h);
            self.size.set((width, height));
        }
    }
}

impl GlContext for WindowedGl {
    fn make_current(&self) -> Result<()> {
        self.ctx
            .make_current(&self.surface)
            .map_err(|e| format!("make_current: {e}"))
    }
    fn gl(&self) -> &glow::Context {
        &self.gl
    }
    fn size(&self) -> (u32, u32) {
        self.size.get()
    }
    fn scale_factor(&self) -> f64 {
        self.window.scale_factor()
    }
    fn present(&self) -> Result<()> {
        self.surface
            .swap_buffers(&self.ctx)
            .map_err(|e| format!("swap_buffers: {e}"))
    }
}
