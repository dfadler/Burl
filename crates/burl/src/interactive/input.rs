//! Window events to commands (phase 6, ticket #20): the key and mouse rows of the app-core table
//! (`discovery/app-core.md` T2.2; `GourceShell::keyPress`, `Gource::keyPress`, `mouseMove`,
//! `mouseClick`, `mouseWheel`) as pure functions, so they test without a display. The winit
//! handler only feeds these and applies what they return.

use crate::frame::Label;
use burl_gl::winit::event::{MouseButton, MouseScrollDelta};
use burl_gl::winit::keyboard::{Key, KeyCode, KeyLocation, NamedKey, PhysicalKey};
use burl_sim::{Button, Command};

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
    /// `R`: `hide_root`.
    ToggleRoot,
    /// `M`: hide the cursor and `hide_mouse`.
    ToggleMouse,
    /// `U`, `D`, `F`, `K`: user names, directory names, file names, the key legend.
    ToggleLabel(Label),
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
    if matches!(key, Key::Named(NamedKey::Tab)) {
        return Some(Action::Sim(NextUser));
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
        "r" | "R" => Action::ToggleRoot,
        "m" | "M" => Action::ToggleMouse,
        "u" | "U" => Action::ToggleLabel(Label::UserNames),
        "d" | "D" => Action::ToggleLabel(Label::DirNames),
        "f" | "F" => Action::ToggleLabel(Label::FileNames),
        "k" | "K" => Action::ToggleLabel(Label::Key),
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

/// A winit button press or release as the sim's [`Command`]; the sim keeps the button and drag
/// state (`Gource::mouseClick`), so nothing is tracked here.
pub fn button_command(b: MouseButton, down: bool) -> Option<Command> {
    let button = match b {
        MouseButton::Left => Button::Left,
        MouseButton::Middle => Button::Middle,
        MouseButton::Right => Button::Right,
        _ => return None,
    };
    Some(Command::Button { button, down })
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
        assert_eq!(act(&ch("r")), Some(Action::ToggleRoot));
        assert_eq!(act(&ch("m")), Some(Action::ToggleMouse));
        for (k, l) in [
            ("u", Label::UserNames),
            ("d", Label::DirNames),
            ("f", Label::FileNames),
            ("k", Label::Key),
        ] {
            assert_eq!(act(&ch(k)), Some(Action::ToggleLabel(l)), "{k}");
        }
        assert_eq!(
            act(&Key::Named(NamedKey::Space)),
            Some(Action::Sim(TogglePause))
        );
        assert_eq!(act(&ch("x")), None);
    }

    /// G10: Gource keys Burl has not ported do nothing (and Z, which Gource itself ignores).
    #[test]
    fn deviation_g10_unported_keys_do_nothing() {
        for k in ["s", "c", "v", "p", "q", "w", "y", "z"] {
            assert_eq!(act(&ch(k)), None, "{k}");
        }
        assert_eq!(act(&Key::Named(NamedKey::F12)), None);
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
    fn buttons_map_to_the_sims_and_tab_selects_the_next_user() {
        assert_eq!(
            button_command(MouseButton::Left, true),
            Some(Command::Button {
                button: Button::Left,
                down: true
            })
        );
        assert_eq!(
            button_command(MouseButton::Middle, false),
            Some(Command::Button {
                button: Button::Middle,
                down: false
            })
        );
        assert_eq!(button_command(MouseButton::Back, true), None);
        assert_eq!(
            act(&Key::Named(NamedKey::Tab)),
            Some(Action::Sim(Command::NextUser))
        );
    }
}
