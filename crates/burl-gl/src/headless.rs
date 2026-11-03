//! Headless context: glutin EGL device display + surfaceless context (no winit, no X, no
//! Wayland), as proven in the #15 spike. On macOS the same code path uses CGL, which is how the
//! fast tier compiles and the developer machine runs it.

use crate::{GlContext, Result};
use glutin::{config::*, context::*, display::*};

#[cfg(target_os = "macos")]
use glutin::api::cgl::context::PossiblyCurrentContext;
#[cfg(not(target_os = "macos"))]
use glutin::api::egl::context::PossiblyCurrentContext;

// Field order is drop order: the glow handle goes before the context, the context before the display.
pub struct HeadlessEgl {
    gl: glow::Context,
    ctx: PossiblyCurrentContext,
    _display: Display,
    size: (u32, u32),
}

impl HeadlessEgl {
    /// A 3.2 core context on the first EGL device (honours Mesa's environment, e.g.
    /// `LIBGL_ALWAYS_SOFTWARE=1`); no Burl-specific device selection (#19 item 5).
    pub fn new(width: u32, height: u32) -> Result<Self> {
        let display = Self::display()?;
        let template = ConfigTemplateBuilder::new()
            .with_surface_type(ConfigSurfaceTypes::empty())
            .with_alpha_size(8)
            .build();
        // SAFETY: `display` is a live glutin display; the config is used only with it.
        let config = unsafe { display.find_configs(template) }
            .map_err(|e| format!("find_configs: {e}"))?
            .next()
            .ok_or("no GL config")?;
        let attrs = ContextAttributesBuilder::new()
            .with_profile(GlProfile::Core)
            .with_context_api(ContextApi::OpenGl(Some(Version::new(3, 2))))
            .build(None);
        // SAFETY: no window handle is passed, so there is nothing that must outlive the context.
        let ctx = unsafe { display.create_context(&config, &attrs) }
            .map_err(|e| format!("create_context (3.2 core, no GLES fallback): {e}"))?;
        let ctx = match ctx {
            #[cfg(not(target_os = "macos"))]
            NotCurrentContext::Egl(c) => c.make_current_surfaceless(),
            #[cfg(target_os = "macos")]
            NotCurrentContext::Cgl(c) => c.make_current_surfaceless(),
        }
        .map_err(|e| format!("make_current_surfaceless: {e}"))?;
        // SAFETY: the context is current on this thread, and `display` outlives the loader calls.
        let gl =
            unsafe { glow::Context::from_loader_function_cstr(|s| display.get_proc_address(s)) };
        Ok(Self {
            gl,
            ctx,
            _display: display,
            size: (width, height),
        })
    }

    #[cfg(not(target_os = "macos"))]
    fn display() -> Result<Display> {
        use glutin::api::egl::{device::Device, display::Display as EglDisplay};
        let device = Device::query_devices()
            .map_err(|e| format!("EGL device query: {e}"))?
            .next()
            .ok_or("no EGL device (is libegl1 + a Mesa driver installed?)")?;
        // SAFETY: a device display has no native display handle to keep alive.
        let display = unsafe { EglDisplay::with_device(&device, None) }
            .map_err(|e| format!("EGL with_device: {e}"))?;
        Ok(Display::Egl(display))
    }

    #[cfg(target_os = "macos")]
    fn display() -> Result<Display> {
        use raw_window_handle::{AppKitDisplayHandle, RawDisplayHandle};
        // SAFETY: the AppKit display handle carries no pointers.
        unsafe {
            Display::new(
                RawDisplayHandle::AppKit(AppKitDisplayHandle::new()),
                DisplayApiPreference::Cgl,
            )
        }
        .map_err(|e| format!("CGL display: {e}"))
    }
}

impl GlContext for HeadlessEgl {
    fn make_current(&self) -> Result<()> {
        // `PossiblyCurrentContext::make_current_surfaceless` is the re-bind for a surfaceless context.
        self.ctx
            .make_current_surfaceless()
            .map_err(|e| format!("make_current_surfaceless: {e}"))
    }
    fn gl(&self) -> &glow::Context {
        &self.gl
    }
    fn size(&self) -> (u32, u32) {
        self.size
    }
    fn scale_factor(&self) -> f64 {
        1.0
    }
    fn present(&self) -> Result<()> {
        Ok(())
    }
}
