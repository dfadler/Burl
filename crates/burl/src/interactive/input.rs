//! Window events to commands (phase 6, ticket #20): the key and mouse rows of the app-core table
//! (`discovery/app-core.md` T2.2; `GourceShell::keyPress`, `Gource::keyPress`, `mouseMove`,
//! `mouseClick`, `mouseWheel`) as pure functions, so they test without a display. The winit
//! handler only feeds these and applies what they return.

use burl_gl::winit::event::{MouseButton, MouseScrollDelta};
use burl_gl::winit::keyboard::{Key, KeyCode, KeyLocation, NamedKey, PhysicalKey};
use burl_sim::Command;

/// What a key press asks of the app. `Sim` goes to the simulation before the next step.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Action {
    Sim(Command),
    /// Esc (works with `--disable-input`).
    Quit,
    /// `T`: `hide_tree`.
    ToggleTree,
    /// `G`: `hide_users`.
    ToggleUsers,
    /// `M`: hide the cursor and `hide_mouse`.
    ToggleMouse,
    /// F11: `toggleWindowFrame`.
    ToggleFrame,
    /// Alt+Return: `toggleFullscreen`.
    ToggleFullscreen,
    /// Return: `GourceShell::next`, honoured only with more than one `[gource]` section.
    NextSection,
}

/// The action for a key press. `key` is the layout key without modifiers (SDL's `keysym.sym`,
/// unaffected by Shift), so letters arrive lowercase. Keypad keys are separate keys in SDL: only
/// `KP -`/`KP +` do anything (zoom), the keypad's digits and punctuation never reach the
/// character rows. `repeat` is an auto-repeat: the shell's keys ignore it, Gource's do not.
/// `alt` is either Alt key.
pub fn key_action(
    key: &Key,
    physical: PhysicalKey,
    location: KeyLocation,
    repeat: bool,
    alt: bool,
    disable_input: bool,
) -> Option<Action> {
    use Command::*;
    if location == KeyLocation::Numpad {
        if disable_input {
            return None;
        }
        return match physical {
            PhysicalKey::Code(KeyCode::NumpadSubtract) => Some(Action::Sim(ZoomIn)),
            PhysicalKey::Code(KeyCode::NumpadAdd) => Some(Action::Sim(ZoomOut)),
            _ => None,
        };
    }
    if !repeat && matches!(key, Key::Named(NamedKey::Escape)) {
        return Some(Action::Quit);
    }
    if disable_input {
        return None;
    }
    if !repeat {
        match key {
            Key::Named(NamedKey::F11) => return Some(Action::ToggleFrame),
            Key::Named(NamedKey::Enter) if alt => return Some(Action::ToggleFullscreen),
            Key::Named(NamedKey::Enter) => return Some(Action::NextSection),
            _ => {}
        }
    }
    if matches!(key, Key::Named(NamedKey::Space)) {
        return Some(Action::Sim(TogglePause));
    }
    let Key::Character(c) = key else {
        return None;
    };
    Some(match c.as_str() {
        "=" | "+" => Action::Sim(Faster),
        "-" => Action::Sim(Slower),
        "." => Action::Sim(TimeScaleUp),
        "," => Action::Sim(TimeScaleDown),
        "/" => Action::Sim(TimeScaleReset),
        "[" => Action::Sim(GravityDown),
        "]" => Action::Sim(GravityUp),
        "n" | "N" => Action::Sim(SkipIdle),
        "t" | "T" => Action::ToggleTree,
        "g" | "G" => Action::ToggleUsers,
        "m" | "M" => Action::ToggleMouse,
        _ => return None,
    })
}

/// Arrow keys held now; the sim polls them every frame (`SDL_GetKeyboardState`), so this is
/// state, not events.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Arrows {
    pub right: bool,
    pub left: bool,
    pub up: bool,
    pub down: bool,
}

impl Arrows {
    /// Track a press or release; returns false when the key is not an arrow.
    pub fn set(&mut self, key: &Key, down: bool) -> bool {
        let slot = match key {
            Key::Named(NamedKey::ArrowRight) => &mut self.right,
            Key::Named(NamedKey::ArrowLeft) => &mut self.left,
            Key::Named(NamedKey::ArrowUp) => &mut self.up,
            Key::Named(NamedKey::ArrowDown) => &mut self.down,
            _ => return false,
        };
        *slot = down;
        true
    }

    pub fn any(&self) -> bool {
        self.right || self.left || self.up || self.down
    }

    pub fn command(&self) -> Option<Command> {
        self.any().then_some(Command::Arrows {
            right: self.right,
            left: self.left,
            up: self.up,
            down: self.down,
        })
    }
}

/// `Gource::mouseWheel` (SDL2): up zooms in, down zooms out, once per event whatever its size.
pub fn wheel_command(d: &MouseScrollDelta) -> Option<Command> {
    let y = match d {
        MouseScrollDelta::LineDelta(_, y) => *y as f64,
        MouseScrollDelta::PixelDelta(p) => p.y,
    };
    if y > 0.0 {
        Some(Command::ZoomIn)
    } else if y < 0.0 {
        Some(Command::ZoomOut)
    } else {
        None
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PointerOut {
    Cmd(Command),
    /// `Gource::grabMouse`.
    Grab(bool),
}

/// Button and drag state of `Gource::mouseClick` / `mouseMove`. Picking (users, files, the
/// slider) is not here yet, so a left press always counts as a click on the background.
#[derive(Debug, Default)]
pub struct Pointer {
    left: bool,
    right: bool,
    middle: bool,
    dragged: bool,
}

impl Pointer {
    pub fn button_pressed(&self) -> bool {
        self.left || self.right || self.middle
    }

    /// `mousedragged || mouseclicked || leftButtonPressed`: `M` is ignored while one holds.
    pub fn busy(&self) -> bool {
        self.dragged || self.left
    }

    pub fn button(&mut self, b: MouseButton, down: bool) -> Vec<PointerOut> {
        use PointerOut::*;
        let mut out = Vec::new();
        match b {
            MouseButton::Left => self.left = down,
            MouseButton::Right => self.right = down,
            MouseButton::Middle => self.middle = down,
            _ => return out,
        }
        if !down {
            if b == MouseButton::Left {
                self.dragged = false;
            }
            if b == MouseButton::Right {
                out.push(Cmd(Command::RightButton(false)));
            }
            // also for the middle button: the C++ leaves the grab on when it is the last one up
            if !self.button_pressed() {
                out.push(Grab(false));
            }
            return out;
        }
        match b {
            MouseButton::Right => {
                out.push(Cmd(Command::RightButton(true)));
                out.push(Grab(true));
            }
            MouseButton::Left => {
                // selectBackground: deselect, take the camera, start dragging
                self.dragged = true;
                out.push(Cmd(Command::DragStart));
                out.push(Grab(true));
            }
            // middle: toggleCameraMode needs the track camera, which the sim does not have yet
            _ => {}
        }
        out
    }

    /// Relative motion while a button is held. The right button rotates and wins over a drag.
    pub fn motion(&self, dx: f32, dy: f32) -> Option<Command> {
        if self.right {
            Some(Command::Rotate { dx, dy })
        } else if self.dragged {
            Some(Command::Drag { dx, dy })
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use burl_gl::winit::dpi::PhysicalPosition;
    use burl_gl::winit::keyboard::SmolStr;

    fn ch(s: &str) -> Key {
        Key::Character(SmolStr::new(s))
    }

    fn act(k: &Key) -> Option<Action> {
        key_action(
            k,
            PhysicalKey::Code(KeyCode::KeyA),
            KeyLocation::Standard,
            false,
            false,
            false,
        )
    }

    #[test]
    fn character_rows() {
        use Command::*;
        for (k, c) in [
            ("=", Faster),
            ("+", Faster),
            ("-", Slower),
            (".", TimeScaleUp),
            (",", TimeScaleDown),
            ("/", TimeScaleReset),
            ("[", GravityDown),
            ("]", GravityUp),
            ("n", SkipIdle),
        ] {
            assert_eq!(act(&ch(k)), Some(Action::Sim(c)), "{k}");
        }
        assert_eq!(act(&ch("t")), Some(Action::ToggleTree));
        assert_eq!(act(&ch("g")), Some(Action::ToggleUsers));
        assert_eq!(act(&ch("m")), Some(Action::ToggleMouse));
        assert_eq!(
            act(&Key::Named(NamedKey::Space)),
            Some(Action::Sim(TogglePause))
        );
        assert_eq!(act(&ch("x")), None);
    }

    #[test]
    fn keypad_zoom_is_not_the_character_row() {
        let kp = |code, k: &Key, disable| {
            key_action(
                k,
                PhysicalKey::Code(code),
                KeyLocation::Numpad,
                false,
                false,
                disable,
            )
        };
        // KP- zooms in and KP+ out, the reverse of what the same characters would do elsewhere
        assert_eq!(
            kp(KeyCode::NumpadSubtract, &ch("-"), false),
            Some(Action::Sim(Command::ZoomIn))
        );
        assert_eq!(
            kp(KeyCode::NumpadAdd, &ch("+"), false),
            Some(Action::Sim(Command::ZoomOut))
        );
        assert_eq!(kp(KeyCode::NumpadDecimal, &ch("."), false), None);
        assert_eq!(kp(KeyCode::NumpadAdd, &ch("+"), true), None);
    }

    #[test]
    fn shell_keys_ignore_repeat_and_escape_ignores_disable_input() {
        let esc = Key::Named(NamedKey::Escape);
        let k = |key: &Key, repeat, alt, dis| {
            key_action(
                key,
                PhysicalKey::Code(KeyCode::KeyA),
                KeyLocation::Standard,
                repeat,
                alt,
                dis,
            )
        };
        assert_eq!(k(&esc, false, false, true), Some(Action::Quit));
        assert_eq!(k(&esc, true, false, false), None);
        let enter = Key::Named(NamedKey::Enter);
        assert_eq!(k(&enter, false, false, false), Some(Action::NextSection));
        assert_eq!(
            k(&enter, false, true, false),
            Some(Action::ToggleFullscreen)
        );
        assert_eq!(k(&enter, true, true, false), None);
        // Gource's own keys do repeat, but not with input disabled
        assert!(k(&ch("="), true, false, false).is_some());
        assert_eq!(k(&ch("="), false, false, true), None);
        assert_eq!(
            k(&Key::Named(NamedKey::F11), false, false, false),
            Some(Action::ToggleFrame)
        );
    }

    #[test]
    fn arrows_are_held_state() {
        let mut a = Arrows::default();
        assert_eq!(a.command(), None);
        assert!(a.set(&Key::Named(NamedKey::ArrowRight), true));
        assert!(a.set(&Key::Named(NamedKey::ArrowLeft), true));
        assert_eq!(
            a.command(),
            Some(Command::Arrows {
                right: true,
                left: true,
                up: false,
                down: false
            })
        );
        a.set(&Key::Named(NamedKey::ArrowRight), false);
        a.set(&Key::Named(NamedKey::ArrowLeft), false);
        assert_eq!(a.command(), None);
        assert!(!a.set(&ch("a"), true));
    }

    #[test]
    fn wheel_sign_only() {
        let line = |y| wheel_command(&MouseScrollDelta::LineDelta(0.0, y));
        assert_eq!(line(1.0), Some(Command::ZoomIn));
        assert_eq!(line(-3.0), Some(Command::ZoomOut));
        assert_eq!(line(0.0), None);
        let px = |y| wheel_command(&MouseScrollDelta::PixelDelta(PhysicalPosition::new(0.0, y)));
        assert_eq!(px(0.5), Some(Command::ZoomIn));
        assert_eq!(px(-0.5), Some(Command::ZoomOut));
    }

    #[test]
    fn left_drag_pans_and_releases_the_grab_with_the_last_button() {
        use PointerOut::*;
        let mut p = Pointer::default();
        assert_eq!(p.motion(3.0, 1.0), None);
        assert_eq!(
            p.button(MouseButton::Left, true),
            [Cmd(Command::DragStart), Grab(true)]
        );
        assert_eq!(p.motion(3.0, 1.0), Some(Command::Drag { dx: 3.0, dy: 1.0 }));
        // the right button joins: it rotates instead, and releasing the left keeps the grab
        p.button(MouseButton::Right, true);
        assert_eq!(
            p.motion(3.0, 1.0),
            Some(Command::Rotate { dx: 3.0, dy: 1.0 })
        );
        assert!(p.button(MouseButton::Left, false).is_empty());
        assert_eq!(
            p.button(MouseButton::Right, false),
            [Cmd(Command::RightButton(false)), Grab(false)]
        );
        assert_eq!(p.motion(3.0, 1.0), None);
    }

    #[test]
    fn middle_button_does_nothing_yet_but_holds_the_grab_until_it_is_last_up() {
        let mut p = Pointer::default();
        assert!(p.button(MouseButton::Middle, true).is_empty());
        p.button(MouseButton::Left, true);
        // a held middle button counts as a pressed button, so the grab stays
        assert!(p.button(MouseButton::Left, false).is_empty());
        assert!(!p.busy());
        assert_eq!(
            p.button(MouseButton::Middle, false),
            [PointerOut::Grab(false)]
        );
    }
}
