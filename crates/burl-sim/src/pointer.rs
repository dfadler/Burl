//! The scripted pointer of the oracle fork (`--debug-pointer-script`,
//! `reference/gource-cpp-dump/README.md`): mouse and Tab events keyed by frame, so the pointer
//! logic of `Gource` can be compared with the fork in the state dump.

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
