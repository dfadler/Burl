//! The scripted pointer of the oracle fork (`--debug-pointer-script`,
//! `reference/gource-cpp-dump/README.md`): mouse and Tab events keyed by frame, so the pointer
//! logic of `Gource` can be compared with the fork in the state dump. Also the pointer logic of
//! `Gource` itself: `mouseMove`, `mouseClick`, `selectBackground` and `mousetrace`
//! (src/gource.cpp), as methods the windowed loop and the script driver both call.

use crate::command::Command;
use crate::sim::{DirId, FileId, Sim, UserId};
use burl_util::bounds::Bounds2D;
use glam::{vec2, Vec2};

/// `MouseCursor::timeout`: the cursor hides after this long without a move or click.
pub const CURSOR_TIMEOUT: f32 = 3.0;

/// SDL button numbers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Button {
    Left,
    Middle,
    Right,
}

impl Button {
    /// `SDL_BUTTON(n)`: the bit in the mouse-state mask.
    pub fn mask(self) -> i32 {
        match self {
            Button::Left => 1,
            Button::Middle => 2,
            Button::Right => 4,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PointerKind {
    Move { x: i32, y: i32 },
    Button { button: Button, down: bool },
    Tab,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PointerEvent {
    pub frame: i32,
    pub kind: PointerKind,
}

/// Parse a pointer script: `FRAME move X Y`, `FRAME down|up l|m|r`, `FRAME key tab`; `#`
/// starts a comment and the frames never decrease.
pub fn parse_script(text: &str) -> Result<Vec<PointerEvent>, String> {
    let mut out = Vec::new();
    let mut last = 0;
    for line in text.lines() {
        let line = line.split('#').next().unwrap_or_default();
        let mut w = line.split_whitespace();
        let Some(first) = w.next() else { continue };
        let bad = |what: &str| format!("invalid {what} in pointer script");
        let frame: i32 = first.parse().map_err(|_| bad("line"))?;
        if frame < last {
            return Err(bad("line"));
        }
        last = frame;
        let kind = match w.next().ok_or_else(|| bad("line"))? {
            "move" => {
                let mut n = || w.next().and_then(|v| v.parse::<i32>().ok());
                match (n(), n()) {
                    (Some(x), Some(y)) => PointerKind::Move { x, y },
                    _ => return Err(bad("move")),
                }
            }
            d @ ("down" | "up") => {
                let button = match w.next() {
                    Some("l") => Button::Left,
                    Some("m") => Button::Middle,
                    Some("r") => Button::Right,
                    _ => return Err(bad("button")),
                };
                PointerKind::Button {
                    button,
                    down: d == "down",
                }
            }
            "key" if w.next() == Some("tab") => PointerKind::Tab,
            "key" => return Err(bad("key")),
            _ => return Err(bad("event")),
        };
        out.push(PointerEvent { frame, kind });
    }
    Ok(out)
}

impl Sim {
    /// `MouseCursor::isVisible`: not hidden, moved or clicked lately, and the pointer is in the window.
    pub fn cursor_visible(&self) -> bool {
        !self.cursor_hidden && self.cursor_idle < CURSOR_TIMEOUT && self.cursor_focus
    }

    /// The pointer entered or left the window (`hasFocus`).
    pub fn set_cursor_focus(&mut self, focus: bool) {
        self.cursor_focus = focus;
    }

    /// The window size in pixels (`display.width/height`), which `mousetrace` projects with.
    pub fn set_display_size(&mut self, w: i32, h: i32) {
        self.s.display_width = w;
        self.s.display_height = h;
    }

    /// `hide_users` / `hide_files` (keys G and F).
    pub fn set_hide(&mut self, users: bool, files: bool) {
        self.s.hide_users = users;
        self.s.hide_files = files;
    }

    /// `hide_mouse` (key M): mouse events are ignored while set.
    pub fn set_hide_mouse(&mut self, hide: bool) {
        self.s.hide_mouse = hide;
    }

    /// `cursor.logic(dt)`: the idle clock, with the wall-clock `dt` of the frame (the dump
    /// driver passes the fixed tick).
    pub fn tick_cursor(&mut self, dt: f32) {
        self.cursor_idle += dt;
    }

    /// `cursor.leftButtonPressed()` and friends, from the mask the events maintain.
    fn button_left(&self) -> bool {
        self.pointer_buttons & Button::Left.mask() != 0
    }

    fn button_any(&self) -> bool {
        self.pointer_buttons & (Button::Left.mask() | Button::Right.mask()) != 0
    }

    /// `Gource::grabMouse`: the cursor hides while grabbed and comes back with a fresh idle clock.
    pub fn grab_mouse(&mut self, grab: bool) {
        self.grab_mouse = grab;
        if self.cursor_hidden == grab {
            return;
        }
        self.cursor_hidden = grab;
        if !grab {
            self.cursor_idle = 0.0;
        }
    }

    /// `Gource::mouseMove` for a motion to window pixel `(x, y)` by `(xrel, yrel)`.
    pub fn mouse_move(&mut self, x: i32, y: i32, xrel: i32, yrel: i32) {
        if self.s.disable_input || self.s.hide_mouse {
            return;
        }
        let mag = vec2(xrel as f32, yrel as f32);
        let right = self.pointer_buttons & Button::Right.mask() != 0;
        // move the camera in the direction of the drag, or rotate the tree with the right button
        if self.mousedragged || right {
            if right {
                self.apply(Command::Rotate {
                    dx: mag.x,
                    dy: mag.y,
                });
            } else {
                self.apply(Command::Drag {
                    dx: mag.x,
                    dy: mag.y,
                });
            }
            return;
        }
        if self.grab_mouse {
            return;
        }
        self.mousepos = vec2(x as f32, y as f32);
        self.cursor_idle = 0.0;
    }

    /// `Gource::mouseClick` for a button going down or up. The slider is not here.
    pub fn mouse_button(&mut self, button: Button, down: bool) {
        if self.s.disable_input || self.s.hide_mouse {
            return;
        }
        // the physical state `SDL_GetMouseState` would report
        if down {
            self.pointer_buttons |= button.mask();
        } else {
            self.pointer_buttons &= !button.mask();
        }
        self.right_button = self.pointer_buttons & Button::Right.mask() != 0;
        // a click stops the cursor being idle
        self.cursor_idle = 0.0;

        if !down {
            if button == Button::Left {
                self.mousedragged = false;
            }
            if matches!(button, Button::Left | Button::Right) && !self.button_any() {
                self.grab_mouse(false);
            }
            return;
        }
        match button {
            // toggleCameraMode: the camera modes are not here yet
            Button::Middle => {}
            Button::Right => self.grab_mouse(true),
            Button::Left => self.mouseclicked = true,
        }
    }

    /// `Gource::selectBackground`: only if the left button is still down when the frame draws.
    fn select_background(&mut self) {
        if !self.button_left() {
            return;
        }
        self.select_user(None);
        self.manual_camera = true;
        self.mousedragged = true;
        self.grab_mouse(true);
    }

    /// The pointer part of `Gource::draw`: `mousetrace` when the cursor is shown, else clear the
    /// hover; then the click flag goes (`mouseclicked = false` at the end of `draw`). Call it
    /// once per frame after `update`, as `draw` follows `logic`.
    pub fn draw_pointer(&mut self) {
        if !self.s.hide_mouse && self.cursor_visible() {
            self.mousetrace();
        } else {
            self.hover_user = None;
            self.hover_file = None;
        }
        self.mouseclicked = false;
    }

    /// The world point under the mouse (`mousetrace`, first lines): the camera looks down -z, so
    /// `cam.z` is negative.
    fn projected_mouse(&self) -> Vec2 {
        let cam = self.camera.pos;
        let (w, h) = (self.s.display_width as f32, self.s.display_height as f32);
        let mut p = vec2(
            -(self.mousepos.x * 2.0 - w) / h,
            1.0 - (2.0 * self.mousepos.y) / h,
        ) * cam.z;
        p.x += cam.x;
        p.y += cam.y;
        p
    }

    /// `RFile::overlaps`: a box of `size/2` around the absolute position (`file.png` is square).
    fn file_overlaps(&self, f: FileId, p: Vec2) -> bool {
        let abs = self.file_abs_pos(f);
        let half = self.file(f).size * 0.5;
        Bounds2D::new(abs - vec2(half, half), abs + vec2(half, half)).contains(p)
    }

    /// The user and the file under `p`. `getItemsAt` returns the whole leaf under the point with
    /// no point test, so users are then tested against their own bounds. Gource takes the first of
    /// a `std::set<QuadItem*>` (heap address order, `D12`); here the leaf's insertion order, which
    /// is name order for users and path order for directories.
    pub fn pick_at(&self, p: Vec2) -> (Option<UserId>, Option<FileId>) {
        let mut user = None;
        if !self.s.hide_users {
            if let Some(tree) = &self.user_tree {
                tree.visit_items_at(p, |u, _| {
                    if user.is_none()
                        && self.user_live(u)
                        && !self.user_is_fading(u)
                        && self.user(u).quad.contains(p)
                    {
                        user = Some(u);
                    }
                });
            }
        }
        let mut file = None;
        if user.is_none() && !self.s.hide_files {
            if let Some(tree) = &self.dir_tree {
                tree.visit_items_at(p, |d: DirId, _| {
                    if !self.dir_live(d) {
                        return;
                    }
                    // the `break` leaves only the file loop: the last directory with a hit wins
                    for &f in &self.dir(d).files {
                        if self.file_live(f) && !self.file(f).hidden && self.file_overlaps(f, p) {
                            file = Some(f);
                            break;
                        }
                    }
                });
            }
        }
        (user, file)
    }

    /// A user's name, for a result of [`Sim::pick_at`].
    pub fn user_name(&self, u: UserId) -> &str {
        &self.user(u).name
    }

    /// `RUser::isFading`.
    pub(crate) fn user_is_fading(&self, u: UserId) -> bool {
        let user = self.user(u);
        user.active_actions.is_empty()
            && user.actions.is_empty()
            && (user.elapsed - user.last_action) > self.s.user_idle_time
    }

    /// `Gource::mousetrace`: hover bookkeeping, then the click.
    fn mousetrace(&mut self) {
        let p = self.projected_mouse();
        let (user, file) = self.pick_at(p);
        // a file under the mouse is only ever found when no user is
        if let Some(f) = file {
            self.hover_user = None;
            self.hover_file = Some(f);
        } else if let Some(u) = user {
            self.hover_file = None;
            self.hover_user = Some(u);
        } else {
            self.hover_file = None;
            self.hover_user = None;
        }

        if self.mouseclicked {
            if let Some(u) = self.hover_user {
                self.camera.lock_on(false);
                self.select_user(Some(u));
            } else if let Some(f) = self.hover_file {
                self.camera.lock_on(false);
                self.select_file(Some(f));
            } else {
                self.select_background();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_every_event_and_rejects_going_back() {
        let s = "# c\n10 move 5 -6\n11 down l # x\n11 up r\n12 key tab\n";
        let v = parse_script(s).unwrap();
        assert_eq!(v.len(), 4);
        assert_eq!(v[0].kind, PointerKind::Move { x: 5, y: -6 });
        assert_eq!(
            v[2].kind,
            PointerKind::Button {
                button: Button::Right,
                down: false
            }
        );
        assert_eq!(v[3].kind, PointerKind::Tab);
        assert!(parse_script("5 key tab\n4 key tab\n").is_err());
        assert!(parse_script("1 key esc\n").is_err());
        assert!(parse_script("1 down x\n").is_err());
        assert!(parse_script("1 move 3\n").is_err());
    }
}
