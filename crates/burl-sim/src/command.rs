//! Input commands (phase 6, ticket #20): the simulation- and camera-affecting rows of
//! `Gource::keyPress`, `zoom` and `mouseMove` (src/gource.cpp) as pure functions, and the
//! `Command` the app's winit layer maps events to. The sim never sees a key or mouse type.

use crate::sim::Sim;
use std::f32::consts::PI;

/// `DEGREES_TO_RADIANS` as the C++ float constant.
const DEG: f32 = PI / 180.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Command {
    /// `=` / `+`: `days_per_second` up.
    Faster,
    /// `-`: `days_per_second` down.
    Slower,
    /// `.`
    TimeScaleUp,
    /// `,`
    TimeScaleDown,
    /// `/`
    TimeScaleReset,
    /// Mouse wheel up, keypad `-`.
    ZoomIn,
    /// Mouse wheel down, keypad `+`.
    ZoomOut,
    /// Space.
    TogglePause,
    /// `N`: the next commit pass skips ahead over idle time.
    SkipIdle,
    /// `[`: force gravity `/= 1.1`.
    GravityDown,
    /// `]`: force gravity `*= 1.1`.
    GravityUp,
    /// The arrow keys held this frame (polled every frame, also paused).
    Arrows {
        right: bool,
        left: bool,
        up: bool,
        down: bool,
    },
    /// Left button down on the background (`selectBackground`): the camera goes manual.
    DragStart,
    /// Mouse motion while dragging on the background (`xrel`, `yrel`).
    Drag { dx: f32, dy: f32 },
    /// Mouse motion while the right button is held: rotate the tree.
    Rotate { dx: f32, dy: f32 },
    /// The right button went down or up (`cursor.rightButtonPressed()`).
    RightButton(bool),
    /// Tab: `selectNextUser`.
    NextUser,
}

impl Sim {
    /// One scripted pointer event (the fork's `dumpPointer`).
    pub fn apply_pointer(&mut self, e: &crate::pointer::PointerEvent) {
        use crate::pointer::PointerKind;
        match e.kind {
            PointerKind::Tab => self.apply(Command::NextUser),
            PointerKind::Move { x, y } => {
                let (px, py) = (self.pointer_x, self.pointer_y);
                self.pointer_x = x;
                self.pointer_y = y;
                self.mouse_move(x, y, x - px, y - py);
            }
            PointerKind::Button { button, down } => self.mouse_button(button, down),
        }
    }
}

/// Keys `=`/`+`: `days_per_second`.
pub fn days_per_second_up(v: f32) -> f32 {
    if v >= 1.0 {
        f32::min(30.0, v.floor() + 1.0)
    } else {
        f32::min(1.0, v * 2.0)
    }
}

/// Key `-`.
pub fn days_per_second_down(v: f32) -> f32 {
    if v > 1.0 {
        f32::max(0.0, v.floor() - 1.0)
    } else {
        f32::max(0.0, v * 0.5)
    }
}

/// Key `.`: `time_scale`.
pub fn time_scale_up(v: f32) -> f32 {
    if v >= 1.0 {
        f32::min(4.0, v.floor() + 1.0)
    } else {
        f32::min(1.0, v * 2.0)
    }
}

/// Key `,`.
pub fn time_scale_down(v: f32) -> f32 {
    if v > 1.0 {
        f32::max(0.0, v.floor() - 1.0)
    } else {
        f32::max(0.25, v * 0.5)
    }
}

/// `[` / `]`: `gGourceForceGravity /= 1.1` / `*= 1.1`; `1.1` is a double, the result narrows.
pub fn force_gravity_step(v: f32, up: bool) -> f32 {
    if up {
        (v as f64 * 1.1) as f32
    } else {
        (v as f64 / 1.1) as f32
    }
}

/// The polled arrow keys of `Gource::logic`: assignments in the order right, left, up, down,
/// so left beats right and down beats up. `None` keeps that component.
pub fn arrow_move(right: bool, left: bool, up: bool, down: bool) -> (Option<f32>, Option<f32>) {
    let (mut x, mut y) = (None, None);
    if right {
        x = Some(10.0);
    }
    if left {
        x = Some(-10.0);
    }
    if up {
        y = Some(-10.0);
    }
    if down {
        y = Some(10.0);
    }
    (x, y)
}

/// `Gource::zoom`: the new camera distance for one step (x1.1), clamped to `[min, max]`.
pub fn zoom_distance(distance: f32, zoom_in: bool, min: f32, max: f32) -> f32 {
    const MULTI: f32 = 1.1;
    if zoom_in {
        let d = distance / MULTI;
        if d < min {
            min
        } else {
            d
        }
    } else {
        let d = distance * MULTI;
        if d > max {
            max
        } else {
            d
        }
    }
}

/// Right-button drag in `mouseMove`: the tree rotation (radians) for a mouse delta. The larger
/// axis wins, `min(1, |mag|/10) * 5` degrees, signed like the delta.
pub fn drag_rotation(dx: f32, dy: f32) -> f32 {
    let m = if dx.abs() > dy.abs() { dx } else { dy };
    let angle = f32::min(1.0, m.abs() / 10.0) * 5.0 * DEG;
    if m < 0.0 {
        -angle
    } else {
        angle
    }
}

/// `updateCamera`: world-space camera move for a drag delta, at camera `z` (`cam_rate`).
pub fn drag_camera_delta(dx: f32, dy: f32, camera_z: f32) -> (f32, f32) {
    let cam_rate = -camera_z / 5000.0;
    (dx * cam_rate * 10.0, dy * cam_rate * 10.0)
}

impl Sim {
    /// Apply one command; the driver calls this before each step.
    pub fn apply(&mut self, c: Command) {
        match c {
            Command::Faster => self.s.days_per_second = days_per_second_up(self.s.days_per_second),
            Command::Slower => {
                self.s.days_per_second = days_per_second_down(self.s.days_per_second)
            }
            Command::TimeScaleUp => self.s.time_scale = time_scale_up(self.s.time_scale),
            Command::TimeScaleDown => self.s.time_scale = time_scale_down(self.s.time_scale),
            Command::TimeScaleReset => self.s.time_scale = 1.0,
            Command::ZoomIn | Command::ZoomOut => {
                self.manual_zoom = true;
                let d = -self.camera.dest.z;
                let d = zoom_distance(
                    d,
                    c == Command::ZoomIn,
                    self.s.camera_zoom_min,
                    self.s.camera_zoom_max,
                );
                self.camera.dest.z = -d;
            }
            Command::TogglePause => self.paused = !self.paused,
            Command::SkipIdle => self.idle_time = self.s.auto_skip_seconds,
            Command::GravityDown | Command::GravityUp => {
                self.force_gravity = force_gravity_step(self.force_gravity, c == Command::GravityUp)
            }
            Command::Arrows {
                right,
                left,
                up,
                down,
            } => {
                let (x, y) = arrow_move(right, left, up, down);
                if let Some(x) = x {
                    self.cursor_move.x = x;
                }
                if let Some(y) = y {
                    self.cursor_move.y = y;
                }
                self.manual_camera |= x.is_some() || y.is_some();
            }
            Command::DragStart => {
                // selectBackground: deselect, then the camera goes manual
                self.select_user(None);
                self.manual_camera = true;
            }
            Command::Drag { dx, dy } => self.cursor_move += glam::vec2(dx, dy),
            Command::Rotate { dx, dy } => {
                self.manual_rotate = true;
                self.rotate_angle = drag_rotation(dx, dy);
            }
            Command::RightButton(down) => self.right_button = down,
            Command::NextUser => self.select_next_user(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn days_per_second_steps() {
        // below 1 it doubles, from 1 it steps by whole days, capped at 30; down mirrors it
        assert_eq!(days_per_second_up(0.1), 0.2);
        assert_eq!(days_per_second_up(0.75), 1.0);
        assert_eq!(days_per_second_up(1.0), 2.0);
        assert_eq!(days_per_second_up(1.5), 2.0);
        assert_eq!(days_per_second_up(30.0), 30.0);
        assert_eq!(days_per_second_down(2.0), 1.0);
        assert_eq!(days_per_second_down(1.0), 0.5);
        assert_eq!(days_per_second_down(2.5), 1.0);
        assert_eq!(days_per_second_down(0.0), 0.0);
    }

    #[test]
    fn time_scale_steps() {
        assert_eq!(time_scale_up(0.25), 0.5);
        assert_eq!(time_scale_up(0.75), 1.0);
        assert_eq!(time_scale_up(1.0), 2.0);
        assert_eq!(time_scale_up(4.0), 4.0);
        assert_eq!(time_scale_down(2.0), 1.0);
        assert_eq!(time_scale_down(1.0), 0.5);
        assert_eq!(time_scale_down(0.25), 0.25);
        assert_eq!(time_scale_down(0.3), 0.25);
    }

    #[test]
    fn zoom_clamps() {
        assert!((zoom_distance(110.0, true, 50.0, 10000.0) - 100.0).abs() < 1e-3);
        assert_eq!(zoom_distance(51.0, true, 50.0, 10000.0), 50.0);
        assert!((zoom_distance(100.0, false, 50.0, 10000.0) - 110.0).abs() < 1e-3);
        assert_eq!(zoom_distance(9500.0, false, 50.0, 10000.0), 10000.0);
    }

    #[test]
    fn arrows_and_gravity_rows() {
        // right then left: left wins; up then down: down wins; nothing held keeps both
        assert_eq!(arrow_move(true, true, false, false), (Some(-10.0), None));
        assert_eq!(
            arrow_move(true, false, true, true),
            (Some(10.0), Some(10.0))
        );
        assert_eq!(arrow_move(false, false, true, false), (None, Some(-10.0)));
        assert_eq!(arrow_move(false, false, false, false), (None, None));
        // `1.1` is a double: 10 * 1.1 narrows to the f32 nearest 11.000000000000002
        assert_eq!(force_gravity_step(10.0, true), 11.0);
        assert!((force_gravity_step(11.0, false) - 10.0).abs() < 1e-5);
    }

    #[test]
    fn drag_rows() {
        // the larger axis wins; full 5 degrees from 10 pixels on; sign follows the delta
        assert!((drag_rotation(10.0, 3.0) - 5.0 * DEG).abs() < 1e-6);
        assert!((drag_rotation(-20.0, 3.0) + 5.0 * DEG).abs() < 1e-6);
        assert!((drag_rotation(1.0, -5.0) + 2.5 * DEG).abs() < 1e-6);
        assert_eq!(drag_camera_delta(1.0, -2.0, -500.0), (1.0, -2.0));
    }
}
