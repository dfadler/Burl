//! The 3D camera's matrices and screen projection (phase 5, ticket #19). Ported from
//! `ZoomCamera::focus`/`look` (src/zoomcamera.cpp), `SDLAppDisplay::mode3D` and `project`
//! (reference/gource-cpp-dump/src/core/display.cpp) and GLU's `gluPerspective`, `gluLookAt` and
//! `gluProject`. The camera position itself is simulation state (`burl-sim`); this only turns it
//! into matrices.

use glam::{DVec4, Mat4, Vec3, Vec4};

/// `ZoomCamera::up`: y points down on screen, so world y grows downwards like pixel rows.
pub const CAMERA_UP: Vec3 = Vec3::new(0.0, -1.0, 0.0);
/// `ZoomCamera::fov` (degrees).
pub const CAMERA_FOV: f32 = 90.0;
/// `ZoomCamera::znear`.
pub const CAMERA_ZNEAR: f32 = 0.1;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Camera {
    pub pos: Vec3,
    pub target: Vec3,
    /// `setMaxDistance(max)` also sets `zfar = max + 1`.
    pub max_distance: f32,
}

impl Camera {
    /// `mode3D(fov, znear, zfar)`: `gluPerspective` with the display's aspect ratio.
    pub fn projection(&self, width: f32, height: f32) -> Mat4 {
        // gluPerspective: cotangent in double, matrix in float.
        let (zn, zf) = (CAMERA_ZNEAR, self.max_distance + 1.0);
        let f = 1.0 / (CAMERA_FOV as f64 * 0.5).to_radians().tan();
        let mut m = Mat4::ZERO;
        m.x_axis.x = (f / (width / height) as f64) as f32;
        m.y_axis.y = f as f32;
        m.z_axis.z = (zf + zn) / (zn - zf);
        m.z_axis.w = -1.0;
        m.w_axis.z = 2.0 * zf * zn / (zn - zf);
        m
    }

    /// `look()`: `gluLookAt(pos, target, up)`.
    pub fn view(&self) -> Mat4 {
        // gluLookAt: rows are side, true up and -forward, then translate by -eye.
        let forward = (self.target - self.pos).normalize();
        let side = forward.cross(CAMERA_UP).normalize();
        let up = side.cross(forward);
        let rot = Mat4::from_cols(
            Vec4::new(side.x, up.x, -forward.x, 0.0),
            Vec4::new(side.y, up.y, -forward.y, 0.0),
            Vec4::new(side.z, up.z, -forward.z, 0.0),
            Vec4::W,
        );
        rot * Mat4::from_translation(-self.pos)
    }

    /// Projection * modelview: the matrix for `Space::World` batches.
    pub fn view_projection(&self, width: f32, height: f32) -> Mat4 {
        self.projection(width, height) * self.view()
    }

    /// `gluProject` followed by `display.project`'s `winY = viewport_height - winY`: pixels with
    /// y down, and window-space depth in `z`. Computed in double like GLU (the GL matrices are
    /// read back with `glGetDoublev`).
    pub fn project(&self, p: Vec3, width: f32, height: f32) -> Vec3 {
        let m = (self.view_projection(width, height)).as_dmat4();
        let c = m * DVec4::new(p.x as f64, p.y as f64, p.z as f64, 1.0);
        // gluProject returns failure (and leaves the outputs alone) when w == 0; a point on the
        // camera plane never reaches the scene, so treat it as the origin.
        if c.w == 0.0 {
            return Vec3::ZERO;
        }
        let (x, y, z) = (c.x / c.w, c.y / c.w, c.z / c.w);
        let win_x = (x * 0.5 + 0.5) * width as f64;
        let win_y = (y * 0.5 + 0.5) * height as f64;
        Vec3::new(
            win_x as f32,
            (height as f64 - win_y) as f32,
            (z * 0.5 + 0.5) as f32,
        )
    }
}
