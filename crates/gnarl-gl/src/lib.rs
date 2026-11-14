//! GL backend (phase 5, ticket #19): the `GlContext` trait, the headless context and the one
//! glow `Renderer`. The only crate allowed to use `unsafe`: every glow call is `unsafe fn`, and
//! each block below states why it holds.

mod headless;
mod renderer;
mod window;

pub use headless::HeadlessEgl;
pub use renderer::Renderer;
pub use window::{monitor_index, sample_attempts, WindowedGl};
pub use winit;

pub type Result<T> = std::result::Result<T, String>;

/// What the renderer needs from a window system: a current GL context and the output size.
/// Implemented by [`HeadlessEgl`] now; the winit + glutin window follows with the windowed path.
pub trait GlContext {
    /// Make the context current on this thread. Contexts are created current.
    fn make_current(&self) -> Result<()>;
    fn gl(&self) -> &glow::Context;
    /// Output size in pixels. Exported frames render at exactly this size.
    fn size(&self) -> (u32, u32);
    /// Logical-to-physical pixel ratio (1.0 headless).
    fn scale_factor(&self) -> f64;
    /// Show the frame (no-op headless).
    fn present(&self) -> Result<()>;
}
