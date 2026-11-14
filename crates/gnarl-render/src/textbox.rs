//! The hover text box (`TextBox`, src/textbox.cpp; drawn at the end of `Gource::draw`,
//! gource.cpp:2688-2709): the name of the user or file under the mouse, and a file's path.

use crate::{quad_batches, Batch, Blend, Font, FontManager, Pass, Pipeline, QuadBuf, Space};
use glam::{Vec2, Vec3, Vec4};

/// `TextBox::max_width_chars`.
const MAX_CHARS: usize = 1024;
/// `TextBox::brightness`, set by `Gource` to 0.5.
const BRIGHTNESS: f32 = 0.5;
/// `TextBox::shadow`.
const SHADOW: Vec2 = Vec2::new(3.0, 3.0);

pub struct TextBox {
    font: Font,
}

/// `TextBox::setPos(pos, true)`: the corner of a `rect` sized box for the mouse at `mouse`,
/// above and left of the cursor, flipped to stay inside the window.
pub fn place(mouse: Vec2, rect: Vec2, font_height: f32, display: Vec2) -> Vec2 {
    let mut c = mouse;
    c.y -= rect.y;
    if c.x + rect.x > display.x {
        if c.x - rect.x - font_height > 0.0 {
            c.x -= rect.x;
        } else {
            c.x = display.x - rect.x;
        }
    }
    if c.y < 0.0 {
        c.y += rect.y + font_height;
    }
    if c.y + rect.y > display.y {
        c.y -= rect.y;
    }
    c
}

impl TextBox {
    /// `size` is `18 * font_scale`.
    pub fn new(mgr: &mut FontManager, size: u32) -> Self {
        Self {
            font: mgr.grab(size),
        }
    }

    /// Appends the shadow, the box and the lines for the mouse at `mouse` in a `display` sized
    /// window. `colour` is the pawn's `getColour()`.
    pub fn draw(
        &mut self,
        mgr: &mut FontManager,
        out: &mut Vec<Batch>,
        lines: &[String],
        colour: Vec3,
        mouse: Vec2,
        display: Vec2,
    ) {
        let lines: Vec<String> = lines
            .iter()
            .map(|l| l.chars().take(MAX_CHARS).collect())
            .collect();
        let step = self.font.size() as f32 + 4.0;
        // `int rect_width`/`rect_height`: the width of the widest line plus 6, 2 plus a row each
        let w = lines
            .iter()
            .map(|l| mgr.width(&self.font, l) as i32 + 6)
            .max()
            .unwrap_or(0);
        let rect = Vec2::new(w as f32, (2 + lines.len() as i32 * step as i32) as f32);
        let corner = place(mouse, rect, step, display);

        let mut quad = |offset: Vec2, colour: Vec4| {
            let mut buf = QuadBuf::default();
            buf.add(None, corner, rect, colour, Vec4::ZERO);
            out.extend(quad_batches(
                Pass::Overlay,
                Pipeline::Plain,
                Blend::Alpha,
                Space::Screen,
                offset.to_array(),
                &buf,
                None,
            ));
        };
        quad(SHADOW, Vec4::new(0.0, 0.0, 0.0, 0.333));
        quad(Vec2::ZERO, (colour * BRIGHTNESS).extend(1.0));

        self.font.set_colour(Vec4::ONE);
        let mut y = 3.0;
        for l in &lines {
            mgr.draw(
                &self.font,
                out,
                (corner.x as i32 + 2) as f32,
                (corner.y as i32) as f32 + y,
                l,
            );
            y += step;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_box_sits_above_left_of_the_cursor_and_flips_inside_the_window() {
        let d = Vec2::new(640.0, 480.0);
        let r = Vec2::new(100.0, 26.0);
        // roomy: the corner is the mouse, raised by the box height
        assert_eq!(
            place(Vec2::new(200.0, 200.0), r, 22.0, d),
            Vec2::new(200.0, 174.0)
        );
        // too far right, room to the left: the box ends at the cursor
        assert_eq!(place(Vec2::new(600.0, 200.0), r, 22.0, d).x, 500.0);
        // too far right, no room to the left: pinned to the right edge
        assert_eq!(
            place(Vec2::new(630.0, 200.0), Vec2::new(630.0, 26.0), 22.0, d).x,
            10.0
        );
        // too close to the top: below the cursor, a text row lower
        assert_eq!(
            place(Vec2::new(200.0, 10.0), r, 22.0, d).y,
            -16.0 + 26.0 + 22.0
        );
        // would leave the bottom after that: back up
        assert_eq!(place(Vec2::new(200.0, 470.0), r, 22.0, d).y, 444.0);
    }
}
